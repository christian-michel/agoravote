import { SUPPORTED_LANGUAGES, useLanguage } from "./LanguageContext";

/** Sélecteur de langue d'interface — partagé entre `Shell.tsx` (écrans
 * publics) et `AdminLayout.tsx` (écrans admin) pour ne pas dupliquer
 * ce contrôle. Une simple liste déroulante native plutôt qu'un
 * composant personnalisé : deux langues aujourd'hui, pas besoin d'un
 * menu riche (cf. principe du projet "pas d'abstraction prématurée").
 */
export function LanguageSwitcher({ className = "" }: { className?: string }) {
  const { lang, setLang, t } = useLanguage();

  return (
    <select
      aria-label={t("common.language")}
      value={lang}
      onChange={(e) => setLang(e.target.value as (typeof SUPPORTED_LANGUAGES)[number]["code"])}
      className={`rounded-md border border-line bg-surface px-2 py-1.5 text-xs text-ink-soft ${className}`}
    >
      {SUPPORTED_LANGUAGES.map((l) => (
        <option key={l.code} value={l.code}>
          {l.code.toUpperCase()}
        </option>
      ))}
    </select>
  );
}
