import { BrowserRouter, Navigate, Route, Routes } from "react-router-dom";
import { AuthProvider, useAuth } from "./auth/AuthContext";
import { Shell } from "./components/Shell";
import Home from "./pages/Home";
import Login from "./pages/Login";
import Register from "./pages/Register";
import AdminHome from "./pages/AdminHome";
import CampaignManage from "./pages/CampaignManage";
import Vote from "./pages/Vote";
import Results from "./pages/Results";

/** Protège les routes d'administration : redirige vers /connexion si
 * personne n'est authentifié. Pendant la vérification initiale du
 * jeton stocké (`status === "checking"`), n'affiche rien plutôt que
 * de flasher la page de connexion puis le contenu protégé. */
function RequireAuth({ children }: { children: React.ReactNode }) {
  const { status } = useAuth();
  if (status === "anonymous") return <Navigate to="/connexion" replace />;
  if (status === "checking") return null;
  return <>{children}</>;
}

export default function App() {
  return (
    <BrowserRouter>
      <AuthProvider>
        <Shell>
          <Routes>
            <Route path="/" element={<Home />} />
            <Route path="/connexion" element={<Login />} />
            <Route path="/inscription" element={<Register />} />
            <Route
              path="/admin"
              element={
                <RequireAuth>
                  <AdminHome />
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
              path="/campagnes/:campaignId/questions/:questionId/voter"
              element={<Vote />}
            />
            <Route
              path="/campagnes/:campaignId/questions/:questionId/resultats"
              element={<Results />}
            />
          </Routes>
        </Shell>
      </AuthProvider>
    </BrowserRouter>
  );
}
