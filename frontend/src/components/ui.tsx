import type { ButtonHTMLAttributes, InputHTMLAttributes, ReactNode } from "react";
import { useLanguage } from "../i18n/LanguageContext";

/** Bouton d'action affirmative (voter, publier, confirmer) — utilise
 * l'unique accent du système de design (cf. DESIGN.md). Pour toute
 * autre action, utiliser `variant="secondary"`, réservant le vert aux
 * actions qui font réellement avancer un processus démocratique. */
export function Button({
  variant = "primary",
  className = "",
  ...props
}: ButtonHTMLAttributes<HTMLButtonElement> & { variant?: "primary" | "secondary" | "danger" }) {
  const base =
    "inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-md px-4 py-2.5 text-sm font-medium transition-colors disabled:cursor-not-allowed disabled:opacity-50";
  const variants = {
    primary: "bg-accent text-white hover:bg-accent-strong",
    secondary: "border border-line bg-surface text-ink hover:bg-paper",
    danger: "border border-danger text-danger hover:bg-danger-soft",
  };
  return <button className={`${base} ${variants[variant]} ${className}`} {...props} />;
}

export function Field({
  label,
  hint,
  error,
  ...props
}: InputHTMLAttributes<HTMLInputElement> & {
  label: string;
  hint?: string;
  error?: string;
}) {
  const id = props.id ?? props.name;
  return (
    <div className="flex flex-col gap-1.5">
      <label htmlFor={id} className="text-sm font-medium text-ink">
        {label}
      </label>
      <input
        id={id}
        className={`rounded-md border bg-surface px-3 py-2 text-sm text-ink placeholder:text-muted focus:border-accent ${
          error ? "border-danger" : "border-line"
        }`}
        {...props}
      />
      {hint && !error && <p className="text-xs text-muted">{hint}</p>}
      {error && <p className="text-xs text-danger">{error}</p>}
    </div>
  );
}

export function Card({ children, className = "" }: { children: ReactNode; className?: string }) {
  return (
    <div className={`rounded-lg border border-line bg-surface p-6 ${className}`}>{children}</div>
  );
}

export function Alert({ kind = "error", children }: { kind?: "error" | "success"; children: ReactNode }) {
  const styles =
    kind === "error"
      ? "border-danger/30 bg-danger-soft text-danger"
      : "border-accent/30 bg-accent-soft text-accent-strong";
  return <div className={`rounded-md border px-4 py-3 text-sm ${styles}`}>{children}</div>;
}

/** Carte statistique du tableau de bord (planche 1) — la puce colorée
 * suit la palette catégorielle fixe (cf. index.css --color-chart-*,
 * skill dataviz), jamais une couleur arbitraire par carte. */
export function StatCard({
  label,
  value,
  icon,
  hue = "var(--color-chart-1)",
}: {
  label: string;
  value: string | number;
  icon: ReactNode;
  hue?: string;
}) {
  return (
    <Card className="flex items-center gap-4">
      <span
        className="flex h-11 w-11 shrink-0 items-center justify-center rounded-xl"
        style={{ background: `color-mix(in srgb, ${hue} 14%, white)`, color: hue }}
      >
        {icon}
      </span>
      <div className="min-w-0">
        <p className="text-2xl font-semibold text-ink">{value}</p>
        <p className="truncate text-sm text-muted">{label}</p>
      </div>
    </Card>
  );
}

/** Libellé de statut (Draft/Published/Closed) — un mot par état,
 * jamais un jargon système (cf. skill de design : nommer ce que la
 * personne comprend, pas comment le système est construit). */
export function StatusBadge({ status }: { status: "Draft" | "Published" | "Closed" }) {
  const { t } = useLanguage();
  const labelKeys: Record<typeof status, "status.draft" | "status.published" | "status.closed"> = {
    Draft: "status.draft",
    Published: "status.published",
    Closed: "status.closed",
  };
  const styles: Record<typeof status, string> = {
    Draft: "bg-[var(--color-status-neutral-soft)] text-ink-soft",
    Published: "bg-[var(--color-status-good-soft)] text-[var(--color-status-good)]",
    Closed: "bg-ink/10 text-ink-soft",
  };
  return (
    <span className={`rounded-full px-2.5 py-1 text-xs font-medium ${styles[status]}`}>
      {t(labelKeys[status])}
    </span>
  );
}
