import { useEffect, useState } from "react";
import { Link, useParams } from "react-router-dom";
import QRCode from "qrcode";
import { api, ApiError, type Campaign, type Question } from "../api/client";
import { useLanguage, pluralize } from "../i18n/LanguageContext";
import { resolveLocalizedText } from "../i18n/content";
import { Alert, Button, Card, StatCard } from "../components/ui";
import { ShareIcon } from "../components/icons";

/**
 * Page d'invitation publique (idée reprise d'une référence externe,
 * cf. docs/DEVLOG.md) : QR code + statistiques de participation en
 * direct + lien à partager, pour inviter à voter sans devoir naviguer
 * le reste du site. Le compte de bulletins vient de `/ballot_count`
 * (brut, pas un résultat calculé) — cf. sa doc côté API pour la
 * distinction avec `/results`.
 */
export default function CampaignInvite() {
  const { t, lang } = useLanguage();
  const { campaignId, questionId } = useParams<{ campaignId: string; questionId: string }>();
  const [campaign, setCampaign] = useState<Campaign | null>(null);
  const [question, setQuestion] = useState<Question | null>(null);
  const [ballotCount, setBallotCount] = useState<number | null>(null);
  const [qrDataUrl, setQrDataUrl] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  const voteUrl =
    campaignId && questionId
      ? `${window.location.origin}/campagnes/${campaignId}/questions/${questionId}/voter`
      : "";

  useEffect(() => {
    if (!campaignId || !questionId) return;
    setLoading(true);
    setError(null);
    (async () => {
      try {
        const [c, f, bc] = await Promise.all([
          api.getCampaign(campaignId),
          api.getForm(campaignId),
          api.getBallotCount(campaignId, questionId),
        ]);
        const q = f.questions.find((item) => item.id === questionId);
        if (!q) {
          setError(t("results.questionNotFound"));
          return;
        }
        setCampaign(c);
        setQuestion(q);
        setBallotCount(bc.count);
      } catch (err) {
        setError(err instanceof ApiError ? err.message : t("invite.loadError"));
      } finally {
        setLoading(false);
      }
    })();
  }, [campaignId, questionId, t]);

  useEffect(() => {
    if (!voteUrl) return;
    QRCode.toDataURL(voteUrl, { width: 256, margin: 1 })
      .then(setQrDataUrl)
      .catch(() => setQrDataUrl(null));
  }, [voteUrl]);

  async function handleCopy() {
    try {
      await navigator.clipboard.writeText(voteUrl);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch {
      // Presse-papiers indisponible (contexte non sécurisé, permission
      // refusée) — l'utilisateur peut toujours sélectionner le champ
      // texte affiché à côté du bouton, pas d'échec bloquant à signaler.
    }
  }

  if (loading) return <p className="text-sm text-muted">{t("common.loading")}</p>;
  if (error || !campaign || !question) {
    return (
      <div className="mx-auto max-w-md">
        <Alert kind="error">{error ?? t("invite.notFound")}</Alert>
      </div>
    );
  }

  const optionCount = "options" in question.question_type ? question.question_type.options.length : null;

  return (
    <div className="mx-auto flex max-w-2xl flex-col gap-6">
      <div className="text-center">
        <h1 className="text-3xl font-semibold text-ink">🗳️ {t("invite.title")}</h1>
        <p className="mt-2 text-sm text-ink-soft">{t("invite.subtitle")}</p>
      </div>

      <Card className="text-center">
        <h2 className="text-xl font-semibold text-ink">{campaign.title}</h2>
        <p className="mt-1 text-sm text-muted">{resolveLocalizedText(question.prompt, lang)}</p>
      </Card>

      <div className="grid gap-4 sm:grid-cols-2">
        {optionCount !== null && (
          <StatCard
            label={pluralize(optionCount, t("invite.propositionsSingular"), t("invite.propositionsPlural"))}
            value={optionCount}
            icon={<span>📝</span>}
          />
        )}
        <StatCard
          label={pluralize(ballotCount ?? 0, t("invite.votesSingular"), t("invite.votesPlural"))}
          value={ballotCount ?? 0}
          icon={<span>👥</span>}
          hue="var(--color-chart-2)"
        />
      </div>

      <Card>
        <div className="grid gap-6 sm:grid-cols-2 sm:items-center">
          <div className="text-center">
            <h3 className="mb-3 font-medium text-ink">📱 {t("invite.scanTitle")}</h3>
            {qrDataUrl && (
              <img
                src={qrDataUrl}
                alt={t("invite.scanTitle")}
                className="mx-auto rounded-md border border-line"
                width={220}
                height={220}
              />
            )}
            <p className="mt-2 text-xs text-muted">{t("invite.scanHint")}</p>
          </div>
          <div className="text-center">
            <h3 className="mb-3 font-medium text-ink">💻 {t("invite.voteNowTitle")}</h3>
            <Link to={`/campagnes/${campaignId}/questions/${questionId}/voter`}>
              <Button className="w-full">{t("invite.voteNowButton")}</Button>
            </Link>
            <p className="mt-2 text-xs text-muted">{t("invite.voteNowHint")}</p>
          </div>
        </div>
      </Card>

      <Card>
        <h3 className="mb-3 text-center font-medium text-ink">🔗 {t("invite.shareTitle")}</h3>
        <div className="flex gap-2">
          <input
            readOnly
            value={voteUrl}
            className="flex-1 rounded-md border border-line bg-paper px-3 py-2 text-xs text-ink-soft"
            onFocus={(e) => e.currentTarget.select()}
          />
          <button
            type="button"
            onClick={handleCopy}
            className="flex shrink-0 items-center gap-1.5 rounded-md border border-line bg-surface px-3 py-2 text-xs text-ink-soft hover:border-accent"
          >
            <ShareIcon width={14} height={14} /> {copied ? t("invite.copied") : t("invite.copy")}
          </button>
        </div>
      </Card>
    </div>
  );
}
