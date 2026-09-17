import { useState, type FormEvent } from "react";
import { Link, useNavigate } from "react-router-dom";
import { useAuth } from "../auth/AuthContext";
import { useLanguage } from "../i18n/LanguageContext";
import { ApiError } from "../api/client";
import { Alert, Button, Card, Field } from "../components/ui";

export default function Login() {
  const { login } = useAuth();
  const { t } = useLanguage();
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
      setError(err instanceof ApiError ? err.message : t("login.genericError"));
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <div className="mx-auto max-w-sm">
      <h1 className="font-display text-2xl font-medium text-ink">{t("login.title")}</h1>
      <Card className="mt-6">
        <form onSubmit={handleSubmit} className="flex flex-col gap-4">
          {error && <Alert>{error}</Alert>}
          <Field
            label={t("login.email")}
            type="email"
            name="email"
            autoComplete="email"
            required
            value={email}
            onChange={(e) => setEmail(e.target.value)}
          />
          <Field
            label={t("login.password")}
            type="password"
            name="password"
            autoComplete="current-password"
            required
            value={password}
            onChange={(e) => setPassword(e.target.value)}
          />
          <Button type="submit" disabled={submitting}>
            {submitting ? t("login.submitting") : t("login.submit")}
          </Button>
        </form>
      </Card>
      <p className="mt-4 text-sm text-muted">
        {t("login.noAccount")}{" "}
        <Link to="/inscription" className="text-accent hover:underline">
          {t("login.createOrganizer")}
        </Link>
      </p>
      <p className="mt-2 text-sm text-muted">
        {t("login.or")}{" "}
        <Link to="/connexion-g1" className="text-accent hover:underline">
          {t("login.g1Link")}
        </Link>{" "}
        {t("login.g1Suffix")}
      </p>
    </div>
  );
}
