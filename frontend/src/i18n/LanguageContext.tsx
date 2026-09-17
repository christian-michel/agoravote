import { createContext, useContext, useMemo, useState, type ReactNode } from "react";
import { fr, type TranslationKey } from "./translations/fr";
import { en } from "./translations/en";

/** Langues d'interface disponibles — en ajouter une = ajouter un
 * fichier `translations/<code>.ts` (`Record<TranslationKey, string>`,
 * vérifié par le compilateur contre `fr.ts`) puis l'enregistrer ici et
 * dans `DICTIONARIES` ci-dessous. Le contenu des CAMPAGNES (questions,
 * options) n'est pas limité à cette liste : `resolveLocalizedText`
 * (cf. `content.ts`) accepte n'importe quel code de langue déjà
 * présent dans les données, cette liste ne borne que l'interface
 * elle-même.
 */
export const SUPPORTED_LANGUAGES = [
  { code: "fr", label: "Français" },
  { code: "en", label: "English" },
] as const;

export type LanguageCode = (typeof SUPPORTED_LANGUAGES)[number]["code"];

export const DEFAULT_LANGUAGE: LanguageCode = "fr";

const DICTIONARIES: Record<LanguageCode, Record<TranslationKey, string>> = { fr, en };

const STORAGE_KEY = "agoravote.lang";

function isSupportedLanguage(value: string): value is LanguageCode {
  return SUPPORTED_LANGUAGES.some((l) => l.code === value);
}

/** Langue initiale : préférence déjà choisie dans ce navigateur ->
 * langue du navigateur si supportée -> français par défaut. Ne lit
 * PAS `User.preferred_language` (champ persisté côté serveur, cf.
 * `agoravote_core::User`) : ce choix reste local à ce navigateur pour
 * l'instant, cf. docs/ROADMAP.md pour la synchronisation
 * multi-appareil non encore câblée. */
function detectInitialLanguage(): LanguageCode {
  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    if (stored && isSupportedLanguage(stored)) return stored;
  } catch {
    // localStorage indisponible (navigation privée stricte, contexte
    // non sécurisé...) — non bloquant, on retombe sur les autres
    // sources de détection ci-dessous.
  }
  const browserLang = navigator.language?.slice(0, 2);
  if (browserLang && isSupportedLanguage(browserLang)) return browserLang;
  return DEFAULT_LANGUAGE;
}

interface LanguageState {
  lang: LanguageCode;
  setLang: (lang: LanguageCode) => void;
  /** Traduction d'une clé statique de l'interface. `vars` permet une
   * interpolation simple (`"Bonjour {name}."` avec `{ name: "Alix" }`)
   * — volontairement minimal (pas de pluralisation intégrée : les
   * quelques cas singulier/pluriel de l'app choisissent la clé
   * appropriée au point d'appel plutôt que d'ajouter un moteur de
   * règles CLDR pour deux langues, cf. principe du projet "pas
   * d'abstraction prématurée"). */
  t: (key: TranslationKey, vars?: Record<string, string | number>) => string;
}

const LanguageContext = createContext<LanguageState | null>(null);

function interpolate(template: string, vars?: Record<string, string | number>): string {
  if (!vars) return template;
  return template.replace(/\{(\w+)\}/g, (match, key: string) =>
    key in vars ? String(vars[key]) : match,
  );
}

export function LanguageProvider({ children }: { children: ReactNode }) {
  const [lang, setLangState] = useState<LanguageCode>(detectInitialLanguage);

  function setLang(next: LanguageCode) {
    setLangState(next);
    try {
      localStorage.setItem(STORAGE_KEY, next);
    } catch {
      // cf. detectInitialLanguage — non bloquant : le choix reste actif
      // pour la session en cours, simplement pas mémorisé.
    }
  }

  const t = useMemo(() => {
    const dict = DICTIONARIES[lang];
    return (key: TranslationKey, vars?: Record<string, string | number>) =>
      interpolate(dict[key] ?? fr[key], vars);
  }, [lang]);

  const value = useMemo(() => ({ lang, setLang, t }), [lang, t]);

  return <LanguageContext.Provider value={value}>{children}</LanguageContext.Provider>;
}

export function useLanguage(): LanguageState {
  const ctx = useContext(LanguageContext);
  if (!ctx) {
    throw new Error("useLanguage doit être utilisé à l'intérieur de <LanguageProvider>");
  }
  return ctx;
}

/** Choisit la forme singulier/pluriel selon `n` — les deux langues
 * supportées aujourd'hui (fr/en) suivent la même règle simple
 * (singulier si n <= 1), donc un seul helper suffit plutôt qu'une
 * règle par langue. */
export function pluralize(n: number, singular: string, plural: string): string {
  return n > 1 ? plural : singular;
}
