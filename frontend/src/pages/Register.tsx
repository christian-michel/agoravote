import { useState, type FormEvent } from "react";
import { Link, useNavigate } from "react-router-dom";
import { useAuth } from "../auth/AuthContext";
import { ApiError } from "../api/client";
import { Alert, Button, Card, Field } from "../components/ui";

/**
 * Organisation par défaut pour ce prototype : l'API n'a pas encore
 * d'écran de création d'organisation (§21, travaux restants) — cf.
 * README backend. Toute inscription rejoint donc la même organisation
 * de démonstration. À remplacer par un vrai choix d'organisation (ou
 * sa création) dès que cette route existera côté API.
 */
const DEMO_ORGANIZATION_ID = "11111111-1111-1111-1111-111111111111";

export default function Register() {
  const { register } = useAuth();
  const navigate = useNavigate();
  const [displayName, setDisplayName] = useState("");
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);

  async function handleSubmit(event: FormEvent) {
    event.preventDefault();
    setError(null);
    setSubmitting(true);
    try {
      await register({
        organizationId: DEMO_ORGANIZATION_ID,
        email,
        password,
        displayName,
      });
      navigate("/admin");
    } catch (err) {
      setError(err instanceof ApiError ? err.message : "L'inscription a échoué.");
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <div className="mx-auto max-w-sm">
      <h1 className="font-display text-2xl font-medium text-ink">Créer un compte</h1>
      <p className="mt-2 text-sm text-muted">
        Un compte organisateur permet de créer et gérer des campagnes.
      </p>
      <Card className="mt-6">
        <form onSubmit={handleSubmit} className="flex flex-col gap-4">
          {error && <Alert>{error}</Alert>}
          <Field
            label="Nom affiché"
            name="displayName"
            autoComplete="name"
            required
            value={displayName}
            onChange={(e) => setDisplayName(e.target.value)}
          />
          <Field
            label="Adresse e-mail"
            type="email"
            name="email"
            autoComplete="email"
            required
            value={email}
            onChange={(e) => setEmail(e.target.value)}
          />
          <Field
            label="Mot de passe"
            type="password"
            name="password"
            autoComplete="new-password"
            required
            minLength={8}
            hint="8 caractères minimum"
            value={password}
            onChange={(e) => setPassword(e.target.value)}
          />
          <Button type="submit" disabled={submitting}>
            {submitting ? "Création…" : "Créer mon compte"}
          </Button>
        </form>
      </Card>
      <p className="mt-4 text-sm text-muted">
        Déjà un compte ?{" "}
        <Link to="/connexion" className="text-accent hover:underline">
          Se connecter
        </Link>
      </p>
    </div>
  );
}
