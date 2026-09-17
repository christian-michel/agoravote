/**
 * Composants de visualisation — SVG inline, sans bibliothèque de
 * graphiques (cf. skill dataviz invoquée pour cette refonte). Palette
 * catégorielle fixe (jamais recyclée par index, cf. index.css
 * --color-chart-1..8) ; marques fines, extrémités arrondies 4px,
 * espace de 2px en couleur de fond entre segments adjacents.
 */
import { useId } from "react";
import { useLanguage } from "../i18n/LanguageContext";

const CHART_HUES = [
  "var(--color-chart-1)",
  "var(--color-chart-2)",
  "var(--color-chart-3)",
  "var(--color-chart-4)",
  "var(--color-chart-5)",
  "var(--color-chart-6)",
  "var(--color-chart-7)",
  "var(--color-chart-8)",
];

export interface BarListItem {
  key: string;
  label: string;
  value: number;
  highlighted?: boolean;
}

/** Liste à barres horizontales, une seule série (teinte accent) — pour
 * "résultats par option" (planche 5). L'option gagnante est repérée
 * par un libellé en gras, jamais par une teinte différente (cf. règle
 * "le texte ne porte jamais la couleur de la donnée"). */
export function BarList({ items, unit = "%" }: { items: BarListItem[]; unit?: string }) {
  const { t } = useLanguage();
  // Longueur normalisée sur le plus grand item affiché, pas sur une
  // échelle 0-100 fixe : l'option en tête occupe toujours la largeur
  // disponible, ce qui rend le classement lisible d'un coup d'œil même
  // quand aucune option n'approche 100% (cas fréquent en vote par
  // approbation, où les pourcentages ne se somment pas à 100). La
  // valeur exacte reste toujours affichée en toutes lettres à côté de
  // chaque barre — jamais seulement suggérée par sa longueur.
  const max = Math.max(1, ...items.map((i) => i.value));
  return (
    <ul className="flex flex-col gap-3">
      {items.map((item) => (
        <li key={item.key}>
          <div className="flex items-center justify-between gap-3 text-sm">
            <span className={item.highlighted ? "font-semibold text-ink" : "text-ink-soft"}>
              {item.label}
              {item.highlighted && t("charts.winnerSuffix")}
            </span>
            <span className="shrink-0 tabular-nums text-muted">
              {item.value.toFixed(1)}
              {unit}
            </span>
          </div>
          <div className="mt-1.5 h-3 w-full overflow-hidden rounded-full bg-paper">
            <div
              className="h-full rounded-full bg-accent transition-all"
              style={{ width: `${Math.min(100, Math.max(0, (item.value / max) * 100))}%` }}
            />
          </div>
        </li>
      ))}
    </ul>
  );
}

export interface DonutSlice {
  key: string;
  label: string;
  value: number;
}

/** Donut catégoriel avec légende (toujours présente à partir de 2
 * séries, cf. skill dataviz) — pour "répartition des votes". */
export function Donut({ slices }: { slices: DonutSlice[] }) {
  const total = slices.reduce((sum, s) => sum + s.value, 0) || 1;
  const radius = 15.5;
  const circumference = 2 * Math.PI * radius;
  // Espace de séparation entre tranches adjacentes, en unités de
  // longueur d'arc (mêmes unités que `circumference`, pas des degrés)
  // — cf. skill dataviz : un espace en couleur de fond, jamais un
  // contour, pour distinguer deux segments qui se touchent.
  const gap = 2;
  let offset = 0;

  return (
    <div className="flex items-center gap-6">
      <svg viewBox="0 0 36 36" className="h-32 w-32 shrink-0 -rotate-90">
        <circle cx="18" cy="18" r={radius} fill="none" stroke="var(--color-paper)" strokeWidth="6" />
        {slices.map((slice, i) => {
          const fraction = slice.value / total;
          const length = Math.max(0, fraction * circumference - gap);
          const dash = `${length} ${circumference - length}`;
          const circle = (
            <circle
              key={slice.key}
              cx="18"
              cy="18"
              r={radius}
              fill="none"
              stroke={CHART_HUES[i % CHART_HUES.length]}
              strokeWidth="6"
              strokeLinecap="round"
              strokeDasharray={dash}
              strokeDashoffset={-offset}
            />
          );
          offset += fraction * circumference;
          return circle;
        })}
      </svg>
      <ul className="flex flex-col gap-2 text-sm">
        {slices.map((slice, i) => (
          <li key={slice.key} className="flex items-center gap-2">
            <span
              className="h-2.5 w-2.5 shrink-0 rounded-full"
              style={{ background: CHART_HUES[i % CHART_HUES.length] }}
            />
            <span className="text-ink-soft">{slice.label}</span>
            <span className="tabular-nums text-muted">
              {((slice.value / total) * 100).toFixed(0)}%
            </span>
          </li>
        ))}
      </ul>
    </div>
  );
}

/** Jauge circulaire à valeur unique (participation) — figure héroïque,
 * pas de légende nécessaire (une seule série, cf. skill dataviz). */
export function ParticipationGauge({
  percent,
  label,
  sublabel,
}: {
  percent: number;
  label: string;
  sublabel?: string;
}) {
  const id = useId();
  const radius = 15.5;
  const circumference = 2 * Math.PI * radius;
  const clamped = Math.min(100, Math.max(0, percent));
  const length = (clamped / 100) * circumference;

  return (
    <div className="flex items-center gap-5">
      <svg viewBox="0 0 36 36" className="h-24 w-24 shrink-0 -rotate-90" role="img" aria-labelledby={id}>
        <title id={id}>{`${label} : ${clamped.toFixed(0)}%`}</title>
        <circle cx="18" cy="18" r={radius} fill="none" stroke="var(--color-paper)" strokeWidth="4" />
        <circle
          cx="18"
          cy="18"
          r={radius}
          fill="none"
          stroke="var(--color-accent)"
          strokeWidth="4"
          strokeLinecap="round"
          strokeDasharray={`${length} ${circumference - length}`}
        />
      </svg>
      <div>
        <p className="text-2xl font-semibold text-ink">{clamped.toFixed(0)}%</p>
        <p className="text-xs text-muted">{label}</p>
        {sublabel && <p className="mt-1 text-xs text-muted">{sublabel}</p>}
      </div>
    </div>
  );
}
