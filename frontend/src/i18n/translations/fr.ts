/**
 * Dictionnaire français — langue de référence du projet (§1.1
 * "multilinguisme natif" du cahier des charges). `TranslationKey`
 * (dérivé de `keyof typeof fr`) est la source de vérité des clés :
 * `en.ts` doit fournir EXACTEMENT le même jeu de clés (vérifié par le
 * compilateur, cf. sa déclaration `Record<TranslationKey, string>`)
 * — une traduction manquante est une erreur de build, pas un texte
 * anglais qui reste silencieusement en français en production.
 */
export const fr = {
  // --- Commun / navigation ---
  "nav.dashboard": "Tableau de bord",
  "nav.campaigns": "Campagnes",
  "nav.modules": "Modules",
  "nav.users": "Utilisateurs",
  "nav.settings": "Paramètres",
  "nav.soon": "Bientôt",
  "nav.soonTitle": "Bientôt disponible",
  "nav.logout": "Se déconnecter",
  "nav.openMenu": "Ouvrir le menu",
  "nav.closeMenu": "Fermer le menu",
  "shell.login": "Se connecter",
  "shell.createAccount": "Créer un compte",
  "common.loading": "Chargement…",
  "common.back": "←",
  "common.language": "Langue",
  "status.draft": "Brouillon",
  "status.published": "Publiée",
  "status.closed": "Clôturée",

  // --- Accueil ---
  "home.title": "Des consultations et des votes que chacun peut vérifier.",
  "home.subtitle":
    "AgoraVote est un outil libre pour créer des sondages, des consultations et des scrutins — avec des méthodes de calcul transparentes et des résultats traçables jusqu'à leur source.",
  "home.organizeTitle": "Organiser un vote",
  "home.organizeBody":
    "Créez une campagne, construisez le questionnaire, publiez-le, puis dépouillez avec la méthode de votre choix.",
  "home.goDashboard": "Aller au tableau de bord",
  "home.createOrganizer": "Créer un compte organisateur",
  "home.participateTitle": "Participer à un vote",
  "home.participateBody":
    "Si on vous a transmis un lien de participation, ouvrez-le directement — aucun compte n'est nécessaire pour voter à une campagne publique.",

  // --- Connexion ---
  "login.title": "Se connecter",
  "login.email": "Adresse e-mail",
  "login.password": "Mot de passe",
  "login.submitting": "Connexion…",
  "login.submit": "Se connecter",
  "login.noAccount": "Pas encore de compte ?",
  "login.createOrganizer": "Créer un compte organisateur",
  "login.or": "Ou",
  "login.g1Link": "se connecter avec un compte Ğ1",
  "login.g1Suffix": "(identité décentralisée optionnelle).",
  "login.genericError": "La connexion a échoué.",

  // --- Inscription ---
  "register.title": "Créer un compte",
  "register.subtitle": "Un compte organisateur permet de créer et gérer des campagnes.",
  "register.displayName": "Nom affiché",
  "register.email": "Adresse e-mail",
  "register.password": "Mot de passe",
  "register.passwordHint": "8 caractères minimum",
  "register.submitting": "Création…",
  "register.submit": "Créer mon compte",
  "register.haveAccount": "Déjà un compte ?",
  "register.login": "Se connecter",
  "register.genericError": "L'inscription a échoué.",

  // --- Connexion Ğ1 ---
  "g1login.title": "Se connecter avec Ğ1",
  "g1login.subtitle":
    "Identité décentralisée optionnelle : prouvez que vous détenez un compte Ğ1v2 en signant un défi avec votre portefeuille (Cesium², Ğecko...), sans jamais communiquer votre phrase de 12 mots à AgoraVote.",
  "g1login.disclaimer":
    "Cette connexion prouve uniquement que vous possédez ce compte Ğ1 — pas que vous êtes membre de la toile de confiance. AgoraVote ne vérifie pas encore l'adhésion à la toile de confiance.",
  "g1login.step1Title": "1. Obtenir un défi à signer",
  "g1login.step1Body":
    "Générez un défi, copiez-le dans votre portefeuille Ğ1, signez-le avec le compte que vous souhaitez lier, puis reportez ci-dessous la clé publique et la signature obtenues.",
  "g1login.generating": "Génération…",
  "g1login.generateChallenge": "Générer un défi",
  "g1login.challengeLabel": "Défi à signer (valide 5 minutes)",
  "g1login.regenerateChallenge": "Régénérer un défi",
  "g1login.step2Title": "2. Fournir la preuve de signature",
  "g1login.publicKeyLabel": "Clé publique Ğ1 (hexadécimal)",
  "g1login.signatureLabel": "Signature du défi (hexadécimal)",
  "g1login.signaturePlaceholder": "signature obtenue depuis votre portefeuille",
  "g1login.verifying": "Vérification…",
  "g1login.verifyAndLogin": "Vérifier et se connecter",
  "g1login.noAccount": "Pas de compte Ğ1 ?",
  "g1login.loginWithPassword": "Se connecter par e-mail et mot de passe",
  "g1login.challengeError": "Impossible d'obtenir un défi, réessayez.",
  "g1login.verifyError": "La connexion a échoué.",

  // --- Tableau de bord admin ---
  "dashboard.title": "Tableau de bord",
  "dashboard.greeting": "Bonjour {name}.",
  "dashboard.statFollowed": "Campagnes suivies",
  "dashboard.statDrafts": "Brouillons",
  "dashboard.statPublished": "Publiées",
  "dashboard.statClosed": "Clôturées",
  "dashboard.newCampaign": "Nouvelle campagne",
  "dashboard.campaignTitleLabel": "Titre de la campagne",
  "dashboard.campaignTitlePlaceholder": "Budget participatif 2027",
  "dashboard.creating": "Création…",
  "dashboard.create": "Créer la campagne",
  "dashboard.recentCampaigns": "Campagnes récentes",
  "dashboard.seeAll": "Voir tout →",
  "dashboard.recentNote":
    "Liste mémorisée dans ce navigateur — pas encore une vraie liste côté serveur (aucune route ne l'expose pour l'instant).",
  "dashboard.noneVisited": "Aucune campagne visitée depuis ce navigateur pour l'instant.",
  "dashboard.createError": "La création a échoué.",

  // --- Liste des campagnes ---
  "campaignsList.title": "Campagnes",
  "campaignsList.subtitle":
    "Campagnes créées ou consultées depuis ce navigateur — pas encore une vraie liste côté serveur (aucune route ne l'expose pour l'instant).",
  "campaignsList.new": "Nouvelle campagne",
  "campaignsList.none": "Aucune campagne visitée depuis ce navigateur pour l'instant.",
  "campaignsList.createdOn": "Créée le {date}",
  "campaignsList.loadError": "Impossible de charger les campagnes.",

  // --- Modules ---
  "modules.title": "Modules",
  "modules.subtitle":
    "Méthodes de vote installées sur cette instance — natives aujourd'hui, chargeables en WASM à terme (§6.1 du cahier des charges).",
  "modules.inputs": "Entrées",
  "modules.parameters": "Paramètres",
  "modules.outputs": "Sorties",
  "modules.loadError": "Impossible de charger les modules.",

  // --- Analyse ---
  "analysis.title": "Analyse et visualisation",
  "analysis.subtitle": "Comparaison des résultats déjà dépouillés pour chaque question de « {title} ».",
  "analysis.viewChart": "graphique",
  "analysis.viewTable": "tableau",
  "analysis.statQuestions": "Question(s) dans le formulaire",
  "analysis.statTallied": "Question(s) dépouillée(s)",
  "analysis.statValidBallots": "Suffrages exprimés au total",
  "analysis.none": "Aucune question de cette campagne n'a encore de résultat calculé — dépouillez au moins une question depuis sa page de campagne pour voir une comparaison ici.",
  "analysis.colOption": "Option",
  "analysis.colBallots": "Suffrages",
  "analysis.colPercent": "%",
  "analysis.loadError": "Impossible de charger l'analyse.",
  "analysis.notFound": "Introuvable.",

  // --- Gestion de campagne ---
  "campaignManage.back": "← Campagnes",
  "campaignManage.createdOn": "Créée le {date}",
  "campaignManage.publish": "Publier la campagne",
  "campaignManage.close": "Clôturer la campagne",
  "campaignManage.formReadyPrefix": "Le formulaire est prêt (",
  "campaignManage.formReadyQuestion": "question",
  "campaignManage.formReadyQuestions": "questions",
  "campaignManage.formReadySuffix":
    "). Publiez la campagne pour commencer à recevoir des votes.",
  "campaignManage.questions": "Questions",
  "campaignManage.analysisLink": "Analyse et visualisation →",
  "campaignManage.openVote": "Ouvrir l'écran de vote citoyen →",
  "campaignManage.openInvite": "Page d'invitation (QR code) →",
  "campaignManage.loadError": "Impossible de charger la campagne.",
  "campaignManage.publishError": "La publication a échoué.",
  "campaignManage.closeError": "La clôture a échoué.",

  // --- Constructeur de formulaire ---
  "formBuilder.libraryTitle": "Bibliothèque de questions",
  "formBuilder.libraryHint": "Cliquez pour ajouter au formulaire.",
  "formBuilder.buildTitle": "Construire le formulaire",
  "formBuilder.buildHint": "Une campagne ne peut être publiée qu'une fois son formulaire créé.",
  "formBuilder.typeSingleChoice": "Choix unique",
  "formBuilder.typeMultipleChoice": "Choix multiple",
  "formBuilder.typeRanking": "Classement",
  "formBuilder.typeMajorityJudgment": "Jugement majoritaire",
  "formBuilder.typeText": "Texte libre",
  "formBuilder.typeNumber": "Nombre",
  "formBuilder.typeScale": "Échelle",
  "formBuilder.moveUp": "Monter",
  "formBuilder.moveDown": "Descendre",
  "formBuilder.removeQuestion": "Retirer la question",
  "formBuilder.promptLabel": "Intitulé de la question",
  "formBuilder.promptLabelEn": "Intitulé (anglais, optionnel)",
  "formBuilder.promptPlaceholder": "Quelle est votre priorité pour cette année ?",
  "formBuilder.optionPlaceholder": "Option {n}",
  "formBuilder.optionPlaceholderEn": "Traduction anglaise (optionnel)",
  "formBuilder.removeOption": "Retirer cette option",
  "formBuilder.addOption": "+ Ajouter une option",
  "formBuilder.maxSelections": "Max. sélections (optionnel)",
  "formBuilder.maxLength": "Longueur max. (optionnel)",
  "formBuilder.min": "Min (optionnel)",
  "formBuilder.max": "Max (optionnel)",
  "formBuilder.scaleMin": "Minimum",
  "formBuilder.scaleMax": "Maximum",
  "formBuilder.submitting": "Création…",
  "formBuilder.submit": "Créer le formulaire",
  "formBuilder.errorPromptRequired": "Chaque question doit avoir un intitulé.",
  "formBuilder.errorNeedsOptions": "La question « {prompt} » doit avoir au moins deux options.",
  "formBuilder.errorScaleBounds": "La question « {prompt} » (échelle) doit avoir un minimum et un maximum.",
  "formBuilder.submitError": "La création du formulaire a échoué.",

  // --- Vote citoyen ---
  "vote.questionOf": "Question {position} sur {count}",
  "vote.submitting": "Envoi…",
  "vote.submit": "Voter",
  "vote.rankingHint": "Cliquez les options dans votre ordre de préférence (1 = préférée).",
  "vote.majorityJudgmentHint": "Attribuez une mention à chaque proposition.",
  "vote.numberPlaceholder": "Votre réponse",
  "vote.numberBoundsBoth": "Entre {min} et {max}",
  "vote.numberBoundsMin": "Minimum {min}",
  "vote.numberBoundsMax": "Maximum {max}",
  "vote.textPlaceholder": "Votre réponse",
  "vote.notPublishedDraft": "Cette campagne n'accepte pas de vote pour le moment (elle n'a pas encore été publiée).",
  "vote.notPublishedClosed": "Cette campagne n'accepte pas de vote pour le moment (elle est clôturée).",
  "vote.seeResults": "Voir les résultats →",
  "vote.confirmedTitle": "Vote enregistré",
  "vote.confirmedBody": "Merci pour votre participation.",
  "vote.notFound": "Cette question n'existe pas dans cette campagne.",
  "vote.loadError": "Impossible de charger cette question.",
  "vote.submitError": "Le vote n'a pas pu être enregistré.",
  "vote.sidebarTitle": "Votre participation est importante !",
  "vote.sidebarBody":
    "Cette consultation utilise une méthode de calcul transparente — le résultat sera expliqué, pas seulement affiché.",

  // --- Résultats ---
  "results.liveTitle": "Résultats en direct",
  "results.noResponses": "Aucune réponse pour le moment.",
  "results.noWinner": "Aucun gagnant",
  "results.share": "Partager",
  "results.expressedRate": "Suffrages exprimés",
  "results.ballotsReceived": "{valid} sur {total} bulletin(s) reçu(s)",
  "results.byOption": "Résultats par option",
  "results.methodVersion": "Méthode {label} v{version}",
  "results.quorumMet": "Quorum atteint",
  "results.quorumNotMet": "Quorum non atteint",
  "results.understandMethod": "Comprendre la méthode",
  "results.collapse": "Réduire",
  "results.seeExplanation": "Voir l'explication",
  "results.notComputedYet": "Aucun résultat n'a encore été calculé pour cette question.",
  "results.loadError": "Impossible de charger les résultats.",
  "results.questionNotFound": "Cette question n'existe pas dans cette campagne.",
  "results.loadQuestionError": "Impossible de charger cette question.",

  // --- Page d'invitation ---
  "invite.title": "Vous êtes invité·e à voter !",
  "invite.subtitle": "Votre avis compte. Prenez part au sondage :",
  "invite.propositionsSingular": "Proposition",
  "invite.propositionsPlural": "Propositions",
  "invite.votesSingular": "Vote enregistré",
  "invite.votesPlural": "Votes enregistrés",
  "invite.scanTitle": "Scanner pour voter",
  "invite.scanHint": "Scannez ce QR code avec votre smartphone.",
  "invite.voteNowTitle": "Voter maintenant",
  "invite.voteNowButton": "Accéder au sondage",
  "invite.voteNowHint": "Cliquez ici pour voter depuis cet appareil.",
  "invite.shareTitle": "Partager ce sondage",
  "invite.copy": "Copier",
  "invite.copied": "Copié !",
  "invite.notFound": "Sondage introuvable.",
  "invite.loadError": "Impossible de charger ce sondage.",

  // --- Dépouillement (choix unique/multiple) ---
  "tally.method": "Méthode de vote",
  "tally.ballotPreview": "Aperçu du bulletin",
  "tally.eligibleVoters": "Électeurs éligibles",
  "tally.quorumPercent": "Quorum (%)",
  "tally.computing": "Calcul…",
  "tally.launch": "Dépouiller",
  "tally.publicResultsLink": "Page de résultats publique →",
  "tally.validBallots": "{valid} suffrage(s) exprimé(s) sur {total}",
  "tally.genericError": "Le dépouillement a échoué.",

  // --- Jugement majoritaire ---
  "mj.description": "Jugement majoritaire — médiane des mentions par option.",
  "mj.mentionsLegend": "Légende des mentions",
  "mj.totalVotesSingular": "vote au total",
  "mj.totalVotesPlural": "votes au total",
  "mj.median": "▼ MÉDIANE",
  "mj.mention1": "Très défavorable",
  "mj.mention2": "Défavorable",
  "mj.mention3": "Sans avis",
  "mj.mention4": "Favorable",
  "mj.mention5": "Très favorable",
  "mj.mention6": "Ne sait pas",

  // --- Réponses brutes ---
  "responses.noneYet": "Aucune réponse pour le moment.",
  "responses.noneRankingYet": "Aucun classement pour le moment.",
  "responses.responseSingular": "réponse",
  "responses.responsePlural": "réponses",
  "responses.rankingSingular": "classement",
  "responses.rankingPlural": "classements",
  "responses.rankingRawNote": "donnée brute, pas de méthode de calcul de vainqueur (hors MVP)",
  "responses.emptyAnswer": "(réponse vide)",
  "responses.statCount": "Réponses",
  "responses.statMean": "Moyenne",
  "responses.statMedian": "Médiane",
  "responses.statMin": "Min",
  "responses.statMax": "Max",
  "responses.unitAnswer": " réponse(s)",
  "responses.loadError": "Impossible de charger les réponses.",

  // --- Composants partagés ---
  "charts.winnerSuffix": " — gagnant",

  // --- Libellés des méthodes de vote natives ---
  "method.majority.label": "Majorité simple",
  "method.majority.description": "L'option qui obtient le plus de suffrages exprimés l'emporte.",
  "method.approval.label": "Vote par approbation",
  "method.approval.description": "Chaque participant peut approuver plusieurs options sans les classer.",
  "method.score.label": "Vote par score",
  "method.score.description":
    "Chaque participant attribue une note à une ou plusieurs options ; la moyenne décide.",
  "method.majorityJudgment.label": "Jugement majoritaire",
  "method.majorityJudgment.description":
    "Chaque participant attribue une mention (de « Très défavorable » à « Très favorable ») à chaque option ; la médiane des mentions décide, départagée par les pourcentages au-dessus/en-dessous.",
} as const;

/** Toute clé de traduction existante — dérivée du dictionnaire
 * français (langue de référence), pas déclarée à la main : ajouter
 * une clé ici la rend automatiquement exigible dans `en.ts`. */
export type TranslationKey = keyof typeof fr;
