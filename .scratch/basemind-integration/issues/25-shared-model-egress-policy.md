# 25: Seam commun de sortie vers le modèle

**What to build:** Avant tout envoi à un modèle, Alex applique une politique commune : pseudonymisation complète pour une API distante, texte original autorisé pour un modèle local, refus pour une route inconnue.

**Blocked by:** 24 (Contrat de route de confidentialité API/local)

**Status:** ready-for-agent

- [ ] Ajouter un seam de sortie partagé par le runtime natif et les providers avant la sérialisation du payload modèle.
- [ ] Pour une API, exécuter Basemind `redact_text`, l'identity registry et le session vault avant l'envoi.
- [ ] Pour un modèle local, conserver le texte et le contexte originaux sans pseudonymisation.
- [ ] Pour une route inconnue, refuser le tour avant tout transport réseau ou appel modèle.
- [ ] Empêcher les chemins Desktop existants d'envoyer directement un prompt hors du seam.
- [ ] Vérifier que la map de réhydratation ne rejoint ni métadonnées ni payload modèle.
