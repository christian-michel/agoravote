//! Backend PostgreSQL — cf. doc du module parent ([`crate`]) et
//! cahier des charges §12.2.
//!
//! Schéma : voir `migrations/0001_init.sql` pour le détail et sa
//! justification (stockage JSONB plutôt que relationnel complet).

use sqlx::postgres::PgPoolOptions;
use sqlx::{Row, Transaction};

use agoravote_core::{Account, Ballot, Campaign, Form, Id, ResultSet, Session, User};

use crate::StoreError;

/// Embarque le contenu de `migrations/` dans le binaire au moment de
/// la compilation (simple lecture de fichiers locaux, ne nécessite
/// PAS de base de données accessible pendant `cargo build` — à ne pas
/// confondre avec les macros `query!`/`query_as!`, volontairement
/// évitées ici, cf. `Cargo.toml`). Appliqué au runtime par
/// [`PgStore::connect`].
static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

#[derive(Clone)]
pub struct PgStore {
    pool: sqlx::PgPool,
}

impl PgStore {
    /// Ouvre un pool de connexions et applique les migrations en
    /// attente. Peut échouer si la base n'est pas joignable (réseau,
    /// identifiants incorrects) ou si une migration est invalide.
    pub async fn connect(database_url: &str) -> Result<Self, StoreError> {
        // 10 connexions : large pour un usage MVP mono-instance, tout
        // en évitant qu'un pic de requêtes concurrentes n'épuise les
        // connexions autorisées par un PostgreSQL par défaut (100).
        // À ajuster si un vrai profil de charge est mesuré un jour.
        let pool = PgPoolOptions::new()
            .max_connections(10)
            .connect(database_url)
            .await?;

        tracing::info!("connexion PostgreSQL établie, application des migrations...");
        MIGRATOR.run(&pool).await?;
        tracing::info!("migrations PostgreSQL à jour");

        Ok(Self { pool })
    }

    pub async fn insert_campaign(&self, campaign: Campaign) -> Result<(), StoreError> {
        sqlx::query(
            "INSERT INTO campaigns (id, data) VALUES ($1, $2)
             ON CONFLICT (id) DO UPDATE SET data = EXCLUDED.data, updated_at = now()",
        )
        .bind(campaign.id)
        .bind(sqlx::types::Json(&campaign))
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_campaign(&self, id: Id) -> Result<Option<Campaign>, StoreError> {
        let row = sqlx::query("SELECT data FROM campaigns WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;
        row_to_entity(row)
    }

    pub async fn update_campaign<F>(&self, id: Id, f: F) -> Result<Option<Campaign>, StoreError>
    where
        F: FnOnce(&mut Campaign) + Send,
    {
        // Lecture-modification-écriture dans une transaction, avec un
        // verrou de ligne (`FOR UPDATE`) : sans cela, deux requêtes
        // concurrentes de publication sur la même campagne pourraient
        // toutes deux lire l'état "brouillon" puis toutes deux écrire
        // un succès, chacune ignorant le travail de l'autre — un bug
        // classique de "perte de mise à jour" (lost update). `FOR
        // UPDATE` fait attendre la seconde transaction que la première
        // ait validé, pour qu'elle reparte d'un état à jour.
        let mut tx: Transaction<'_, sqlx::Postgres> = self.pool.begin().await?;

        let row = sqlx::query("SELECT data FROM campaigns WHERE id = $1 FOR UPDATE")
            .bind(id)
            .fetch_optional(&mut *tx)
            .await?;

        let Some(row) = row else {
            tx.rollback().await?;
            return Ok(None);
        };

        let sqlx::types::Json(mut campaign): sqlx::types::Json<Campaign> = row.try_get("data")?;
        f(&mut campaign);

        sqlx::query("UPDATE campaigns SET data = $2, updated_at = now() WHERE id = $1")
            .bind(id)
            .bind(sqlx::types::Json(&campaign))
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;
        Ok(Some(campaign))
    }

    pub async fn insert_form(&self, form: Form) -> Result<(), StoreError> {
        sqlx::query(
            "INSERT INTO forms (id, campaign_id, data) VALUES ($1, $2, $3)
             ON CONFLICT (id) DO UPDATE SET data = EXCLUDED.data, campaign_id = EXCLUDED.campaign_id",
        )
        .bind(form.id)
        .bind(form.campaign_id)
        .bind(sqlx::types::Json(&form))
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    #[allow(dead_code)]
    pub async fn get_form(&self, id: Id) -> Result<Option<Form>, StoreError> {
        let row = sqlx::query("SELECT data FROM forms WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;
        row_to_entity(row)
    }

    pub async fn form_for_campaign(&self, campaign_id: Id) -> Result<Option<Form>, StoreError> {
        let row = sqlx::query("SELECT data FROM forms WHERE campaign_id = $1 LIMIT 1")
            .bind(campaign_id)
            .fetch_optional(&self.pool)
            .await?;
        row_to_entity(row)
    }

    pub async fn add_ballot(&self, ballot: Ballot) -> Result<(), StoreError> {
        sqlx::query(
            "INSERT INTO ballots (id, campaign_id, question_id, data, cast_at)
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(ballot.id)
        .bind(ballot.campaign_id)
        .bind(ballot.question_id)
        .bind(sqlx::types::Json(&ballot))
        .bind(ballot.cast_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn ballots_for_question(
        &self,
        campaign_id: Id,
        question_id: Id,
    ) -> Result<Vec<Ballot>, StoreError> {
        let rows = sqlx::query(
            "SELECT data FROM ballots WHERE campaign_id = $1 AND question_id = $2 ORDER BY cast_at",
        )
        .bind(campaign_id)
        .bind(question_id)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|row| {
                let sqlx::types::Json(ballot): sqlx::types::Json<Ballot> = row.try_get("data")?;
                Ok(ballot)
            })
            .collect()
    }

    pub async fn store_result(&self, result: ResultSet) -> Result<(), StoreError> {
        sqlx::query(
            "INSERT INTO result_sets (question_id, campaign_id, data, computed_at)
             VALUES ($1, $2, $3, $4)
             ON CONFLICT (question_id) DO UPDATE SET
                data = EXCLUDED.data,
                campaign_id = EXCLUDED.campaign_id,
                computed_at = EXCLUDED.computed_at",
        )
        .bind(result.question_id)
        .bind(result.campaign_id)
        .bind(sqlx::types::Json(&result))
        .bind(result.computed_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_result(&self, question_id: Id) -> Result<Option<ResultSet>, StoreError> {
        let row = sqlx::query("SELECT data FROM result_sets WHERE question_id = $1")
            .bind(question_id)
            .fetch_optional(&self.pool)
            .await?;
        row_to_entity(row)
    }

    // --- Authentification (§13) ---

    pub async fn insert_user(&self, user: User) -> Result<(), StoreError> {
        sqlx::query(
            "INSERT INTO users (id, data) VALUES ($1, $2)
             ON CONFLICT (id) DO UPDATE SET data = EXCLUDED.data",
        )
        .bind(user.id)
        .bind(sqlx::types::Json(&user))
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_user(&self, id: Id) -> Result<Option<User>, StoreError> {
        let row = sqlx::query("SELECT data FROM users WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;
        row_to_entity(row)
    }

    pub async fn insert_account(&self, account: Account) -> Result<(), StoreError> {
        let result = sqlx::query(
            "INSERT INTO accounts (id, user_id, email, password_hash, created_at)
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(account.id)
        .bind(account.user_id)
        .bind(&account.email)
        .bind(&account.password_hash)
        .bind(account.created_at)
        .execute(&self.pool)
        .await;

        match result {
            Ok(_) => Ok(()),
            Err(err) => Err(map_unique_violation(err)),
        }
    }

    pub async fn get_account_by_email(&self, email: &str) -> Result<Option<Account>, StoreError> {
        let row = sqlx::query(
            "SELECT id, user_id, email, password_hash, created_at FROM accounts WHERE email = $1",
        )
        .bind(email)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|row| Account {
            id: row.get("id"),
            user_id: row.get("user_id"),
            email: row.get("email"),
            password_hash: row.get("password_hash"),
            created_at: row.get("created_at"),
        }))
    }

    pub async fn insert_session(&self, session: Session) -> Result<(), StoreError> {
        sqlx::query(
            "INSERT INTO sessions (token_hash, user_id, organization_id, created_at, expires_at)
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(&session.token_hash)
        .bind(session.user_id)
        .bind(session.organization_id)
        .bind(session.created_at)
        .bind(session.expires_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_session(&self, token_hash: &str) -> Result<Option<Session>, StoreError> {
        let row = sqlx::query(
            "SELECT token_hash, user_id, organization_id, created_at, expires_at
             FROM sessions WHERE token_hash = $1",
        )
        .bind(token_hash)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|row| Session {
            token_hash: row.get("token_hash"),
            user_id: row.get("user_id"),
            organization_id: row.get("organization_id"),
            created_at: row.get("created_at"),
            expires_at: row.get("expires_at"),
        }))
    }

    pub async fn delete_session(&self, token_hash: &str) -> Result<(), StoreError> {
        sqlx::query("DELETE FROM sessions WHERE token_hash = $1")
            .bind(token_hash)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}

/// Convertit une violation de contrainte `UNIQUE` PostgreSQL (code
/// d'erreur `23505`) en [`StoreError::EmailAlreadyExists`], et
/// laisse passer toute autre erreur telle quelle. Isolé dans une
/// fonction pour ne l'écrire qu'une fois — utile si d'autres colonnes
/// uniques apparaissent un jour (ex : un identifiant Ğ1, cf. addendum
/// v0.3 du cahier des charges).
fn map_unique_violation(err: sqlx::Error) -> StoreError {
    if let sqlx::Error::Database(ref db_err) = err {
        if db_err.code().as_deref() == Some("23505") {
            return StoreError::EmailAlreadyExists;
        }
    }
    StoreError::Database(err)
}

/// Désérialise la colonne `data` (JSONB) d'une ligne optionnelle en
/// entité du domaine. Factorisé car ce motif se répète pour toutes
/// les lectures simples (campagne, formulaire, résultat) de ce fichier.
fn row_to_entity<T>(row: Option<sqlx::postgres::PgRow>) -> Result<Option<T>, StoreError>
where
    T: serde::de::DeserializeOwned,
{
    match row {
        None => Ok(None),
        Some(row) => {
            let sqlx::types::Json(entity): sqlx::types::Json<T> = row.try_get("data")?;
            Ok(Some(entity))
        }
    }
}
