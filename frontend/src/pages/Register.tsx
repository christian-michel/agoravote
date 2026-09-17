import { useState, type FormEvent } from "react";
import { Link, useNavigate } from "react-router-dom";
import { useAuth } from "../auth/AuthContext";
import { useLanguage } from "../i18n/LanguageContext";
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
  const { t } = useLanguage();
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
      setError(err instanceof ApiError ? err.message : t("register.genericError"));
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <div className="mx-auto max-w-sm">
      <h1 className="font-display text-2xl font-medium text-ink">{t("register.title")}</h1>
      <p className="mt-2 text-sm text-muted">{t("register.subtitle")}</p>
      <Card className="mt-6">
        <form onSubmit={handleSubmit} className="flex flex-col gap-4">
          {error && <Alert>{error}</Alert>}
          <Field
            label={t("register.displayName")}
            name="displayName"
            autoComplete="name"
            required
            value={displayName}
            onChange={(e) => setDisplayName(e.target.value)}
          />
          <Field
            label={t("register.email")}
            type="email"
            name="email"
            autoComplete="email"
            required
            value={email}
            onChange={(e) => setEmail(e.target.value)}
          />
          <Field
            label={t("register.password")}
            type="password"
            name="password"
            autoComplete="new-password"
            required
            minLength={8}
            hint={t("register.passwordHint")}
            value={password}
            onChange={(e) => setPassword(e.target.value)}
          />
          <Button type="submit" disabled={submitting}>
            {submitting ? t("register.submitting") : t("register.submit")}
          </Button>
        </form>
      </Card>
      <p className="mt-4 text-sm text-muted">
        {t("register.haveAccount")}{" "}
        <Link to="/connexion" className="text-accent hover:underline">
          {t("register.login")}
        </Link>
      </p>
    </div>
  );
}
