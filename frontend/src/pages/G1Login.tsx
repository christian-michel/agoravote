import { useState, type FormEvent } from "react";
import { Link, useNavigate } from "react-router-dom";
import { useAuth } from "../auth/AuthContext";
import { useLanguage } from "../i18n/LanguageContext";
import { api, ApiError } from "../api/client";
import { Alert, Button, Card, Field } from "../components/ui";

/**
 * Organisation par défaut pour ce prototype — même limite que
 * `Register.tsx` (pas encore d'écran de création/choix d'organisation,
 * §21 travaux restants). Un compte Ğ1 nouvellement provisionné rejoint
 * cette même organisation de démonstration.
 */
const DEMO_ORGANIZATION_ID = "11111111-1111-1111-1111-111111111111";

/**
 * Connexion par identité Ğ1v2 optionnelle (planche 17 de l'addendum
 * v0.3, « Identité décentralisée et trajectoire Web3 »).
 *
 * ## Pourquoi une saisie manuelle, pas une extension de portefeuille
 *
 * AgoraVote n'intègre aujourd'hui aucune extension de portefeuille Ğ1
 * (Cesium², Ğecko...) : ce prototype demande à l'utilisateur de copier
 * le défi dans son portefeuille, d'y coller la signature obtenue, puis
 * de la reporter ici. C'est délibérément plus lourd qu'une future
 * intégration native (`window.g1` ou équivalent, hors périmètre
 * actuel), mais chaque étape est réellement vérifiée côté serveur
 * (`agoravote_g1::verify_signature`, une vraie signature ed25519, pas
 * une simulation) — cf. `docs/G1_INTEGRATION.md` §4 et
 * `crates/agoravote-g1/README.md` pour le statut de ce module.
 *
 * ## Ce que cette connexion prouve, et ce qu'elle NE prouve PAS
 *
 * Signer ce défi prouve la possession de la clé privée correspondant
 * à la clé publique fournie — rien de plus. Ce n'est PAS une preuve
 * d'appartenance à la toile de confiance Ğ1 (un compte non-membre
 * peut très bien signer ce défi) : cette vérification-là exigerait une
 * requête réseau vers un nœud Ğ1v2 réel, non câblée dans ce
 * déploiement faute de pouvoir la tester de bout en bout ici (cf.
 * `docs/G1_INTEGRATION.md` §5). Le dire clairement à l'écran plutôt
 * que de laisser croire à une garantie qui n'existe pas encore.
 */
export default function G1Login() {
  const { loginWithG1 } = useAuth();
  const { t } = useLanguage();
  const navigate = useNavigate();

  const [challengeMessage, setChallengeMessage] = useState<string | null>(null);
  const [requestingChallenge, setRequestingChallenge] = useState(false);
  const [challengeError, setChallengeError] = useState<string | null>(null);

  const [publicKeyHex, setPublicKeyHex] = useState("");
  const [signatureHex, setSignatureHex] = useState("");
  const [verifyError, setVerifyError] = useState<string | null>(null);
  const [verifying, setVerifying] = useState(false);

  async function requestChallenge() {
    setChallengeError(null);
    setRequestingChallenge(true);
    try {
      const challenge = await api.g1Challenge();
      setChallengeMessage(challenge.message);
    } catch (err) {
      setChallengeError(
        err instanceof ApiError ? err.message : t("g1login.challengeError"),
      );
    } finally {
      setRequestingChallenge(false);
    }
  }

  async function handleVerify(event: FormEvent) {
    event.preventDefault();
    if (!challengeMessage) return;
    setVerifyError(null);
    setVerifying(true);
    try {
      await loginWithG1({
        organizationId: DEMO_ORGANIZATION_ID,
        publicKeyHex: publicKeyHex.trim(),
        signatureHex: signatureHex.trim(),
        message: challengeMessage,
      });
      navigate("/admin");
    } catch (err) {
      // Message volontairement générique côté API (cf.
      // routes.rs::unauthorized_g1) : signature invalide ou défi
      // expiré produisent la même réponse, on l'affiche telle quelle.
      setVerifyError(err instanceof ApiError ? err.message : t("g1login.verifyError"));
    } finally {
      setVerifying(false);
    }
  }

  return (
    <div className="mx-auto max-w-lg">
      <h1 className="font-display text-2xl font-medium text-ink">{t("g1login.title")}</h1>
      <p className="mt-2 text-sm text-muted">{t("g1login.subtitle")}</p>
      <div className="mt-3 rounded-md border border-line bg-paper px-4 py-3 text-xs text-muted">
        {t("g1login.disclaimer")}
      </div>

      <Card className="mt-6">
        <div className="flex flex-col gap-4">
          <div>
            <p className="text-sm font-medium text-ink">{t("g1login.step1Title")}</p>
            <p className="mt-1 text-sm text-muted">{t("g1login.step1Body")}</p>
          </div>

          {challengeError && <Alert>{challengeError}</Alert>}

          {!challengeMessage ? (
            <Button
              type="button"
              variant="secondary"
              disabled={requestingChallenge}
              onClick={requestChallenge}
            >
              {requestingChallenge ? t("g1login.generating") : t("g1login.generateChallenge")}
            </Button>
          ) : (
            <div className="flex flex-col gap-1.5">
              <label className="text-sm font-medium text-ink" htmlFor="g1-challenge">
                {t("g1login.challengeLabel")}
              </label>
              <textarea
                id="g1-challenge"
                readOnly
                rows={3}
                value={challengeMessage}
                className="rounded-md border border-line bg-paper px-3 py-2 font-mono text-xs text-ink"
                onFocus={(e) => e.currentTarget.select()}
              />
              <button
                type="button"
                onClick={requestChallenge}
                disabled={requestingChallenge}
                className="self-start text-xs text-accent hover:underline"
              >
                {requestingChallenge ? t("g1login.generating") : t("g1login.regenerateChallenge")}
              </button>
            </div>
          )}
        </div>
      </Card>

      {challengeMessage && (
        <Card className="mt-4">
          <form onSubmit={handleVerify} className="flex flex-col gap-4">
            <p className="text-sm font-medium text-ink">{t("g1login.step2Title")}</p>

            {verifyError && <Alert>{verifyError}</Alert>}

            <Field
              label={t("g1login.publicKeyLabel")}
              name="publicKeyHex"
              required
              placeholder="ex : d43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27"
              value={publicKeyHex}
              onChange={(e) => setPublicKeyHex(e.target.value)}
            />

            <div className="flex flex-col gap-1.5">
              <label className="text-sm font-medium text-ink" htmlFor="g1-signature">
                {t("g1login.signatureLabel")}
              </label>
              <textarea
                id="g1-signature"
                required
                rows={2}
                value={signatureHex}
                onChange={(e) => setSignatureHex(e.target.value)}
                className="rounded-md border border-line bg-surface px-3 py-2 font-mono text-xs text-ink placeholder:text-muted focus:border-accent"
                placeholder={t("g1login.signaturePlaceholder")}
              />
            </div>

            <Button type="submit" disabled={verifying}>
              {verifying ? t("g1login.verifying") : t("g1login.verifyAndLogin")}
            </Button>
          </form>
        </Card>
      )}

      <p className="mt-4 text-sm text-muted">
        {t("g1login.noAccount")}{" "}
        <Link to="/connexion" className="text-accent hover:underline">
          {t("g1login.loginWithPassword")}
        </Link>
      </p>
    </div>
  );
}
