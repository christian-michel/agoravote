import { useState, type FormEvent } from "react";
import { Link, useNavigate } from "react-router-dom";
import { useAuth } from "../auth/AuthContext";
import { ApiError } from "../api/client";
import { Alert, Button, Card, Field } from "../components/ui";

export default function Login() {
  const { login } = useAuth();
  const navigate = useNavigate();
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);

  async function handleSubmit(event: FormEvent) {
    event.preventDefault();
    setError(null);
    setSubmitting(true);
    try {
      await login(email, password);
      navigate("/admin");
    } catch (err) {
      // Le message renvoyé par l'API est volontairement générique
      // ("email ou mot de passe incorrect") pour ne pas permettre de
      // deviner quels emails ont un compte — cf. docs/SECURITY.md §7
      // côté backend. On l'affiche tel quel, sans le reformuler.
      setError(err instanceof ApiError ? err.message : "La connexion a échoué.");
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <div className="mx-auto max-w-sm">
      <h1 className="font-display text-2xl font-medium text-ink">Se connecter</h1>
      <Card className="mt-6">
        <form onSubmit={handleSubmit} className="flex flex-col gap-4">
          {error && <Alert>{error}</Alert>}
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
            autoComplete="current-password"
            required
            value={password}
            onChange={(e) => setPassword(e.target.value)}
          />
          <Button type="submit" disabled={submitting}>
            {submitting ? "Connexion…" : "Se connecter"}
          </Button>
        </form>
      </Card>
      <p className="mt-4 text-sm text-muted">
        Pas encore de compte ?{" "}
        <Link to="/inscription" className="text-accent hover:underline">
          Créer un compte organisateur
        </Link>
      </p>
    </div>
  );
}
