import { Link } from "react-router-dom";
import { useAuth } from "../auth/AuthContext";
import { Button, Card } from "../components/ui";

export default function Home() {
  const { status } = useAuth();

  return (
    <div className="flex flex-col gap-10">
      <div className="max-w-2xl">
        <h1 className="font-display text-4xl font-medium leading-tight text-ink">
          Des consultations et des votes que chacun peut vérifier.
        </h1>
        <p className="mt-4 text-base text-ink-soft">
          AgoraVote est un outil libre pour créer des sondages, des consultations
          et des scrutins — avec des méthodes de calcul transparentes et des
          résultats traçables jusqu'à leur source.
        </p>
      </div>

      <div className="grid gap-6 sm:grid-cols-2">
        <Card>
          <h2 className="font-display text-xl font-medium text-ink">Organiser un vote</h2>
          <p className="mt-2 text-sm text-ink-soft">
            Créez une campagne, construisez le questionnaire, publiez-le, puis
            dépouillez avec la méthode de votre choix.
          </p>
          <div className="mt-4">
            {status === "authenticated" ? (
              <Link to="/admin">
                <Button>Aller au tableau de bord</Button>
              </Link>
            ) : status === "anonymous" ? (
              <Link to="/inscription">
                <Button>Créer un compte organisateur</Button>
              </Link>
            ) : null}
          </div>
        </Card>

        <Card>
          <h2 className="font-display text-xl font-medium text-ink">Participer à un vote</h2>
          <p className="mt-2 text-sm text-ink-soft">
            Si on vous a transmis un lien de participation, ouvrez-le
            directement — aucun compte n'est nécessaire pour voter à une
            campagne publique.
          </p>
        </Card>
      </div>
    </div>
  );
}
