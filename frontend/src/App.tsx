import { BrowserRouter, Navigate, Route, Routes } from "react-router-dom";
import { AuthProvider, useAuth } from "./auth/AuthContext";
import { Shell } from "./components/Shell";
import { AdminLayout } from "./components/AdminLayout";
import Home from "./pages/Home";
import Login from "./pages/Login";
import Register from "./pages/Register";
import AdminHome from "./pages/AdminHome";
import CampaignsList from "./pages/CampaignsList";
import ModulesPage from "./pages/ModulesPage";
import CampaignManage from "./pages/CampaignManage";
import Analysis from "./pages/Analysis";
import Vote from "./pages/Vote";
import Results from "./pages/Results";

/** Protège les routes d'administration : redirige vers /connexion si
 * personne n'est authentifié. Pendant la vérification initiale du
 * jeton stocké (`status === "checking"`), n'affiche rien plutôt que
 * de flasher la page de connexion puis le contenu protégé. Enveloppe
 * dans `AdminLayout` (nav latérale, planches 1-3/5-6 du §10) plutôt
 * que `Shell` (nav du haut, réservée aux écrans publics/citoyens). */
function RequireAuth({ children }: { children: React.ReactNode }) {
  const { status } = useAuth();
  if (status === "anonymous") return <Navigate to="/connexion" replace />;
  if (status === "checking") return null;
  return <AdminLayout>{children}</AdminLayout>;
}

export default function App() {
  return (
    <BrowserRouter>
      <AuthProvider>
        <Routes>
          <Route path="/" element={<Shell><Home /></Shell>} />
          <Route path="/connexion" element={<Shell><Login /></Shell>} />
          <Route path="/inscription" element={<Shell><Register /></Shell>} />
          <Route
            path="/admin"
            element={
              <RequireAuth>
                <AdminHome />
              </RequireAuth>
            }
          />
          <Route
            path="/admin/campagnes"
            element={
              <RequireAuth>
                <CampaignsList />
              </RequireAuth>
            }
          />
          <Route
            path="/admin/modules"
            element={
              <RequireAuth>
                <ModulesPage />
              </RequireAuth>
            }
          />
          <Route
            path="/admin/campagnes/:campaignId"
            element={
              <RequireAuth>
                <CampaignManage />
              </RequireAuth>
            }
          />
          <Route
            path="/admin/campagnes/:campaignId/analyse"
            element={
              <RequireAuth>
                <Analysis />
              </RequireAuth>
            }
          />
          <Route
            path="/campagnes/:campaignId/questions/:questionId/voter"
            element={<Shell><Vote /></Shell>}
          />
          <Route
            path="/campagnes/:campaignId/questions/:questionId/resultats"
            element={<Shell><Results /></Shell>}
          />
        </Routes>
      </AuthProvider>
    </BrowserRouter>
  );
}
