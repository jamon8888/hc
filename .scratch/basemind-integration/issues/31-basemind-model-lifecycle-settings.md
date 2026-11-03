# 31: Cycle de vie des modèles Basemind et Settings

**What to build:** L'utilisateur peut gérer les modèles Basemind requis pour la protection PII avec téléchargement vérifié, cache persistant, statut clair et refus propre en cas de modèle absent.

**Blocked by:** None (can start immediately)

**Status:** ready-for-agent

- [ ] Implémenter le téléchargement lazy, la vérification de taille et le digest des modèles.
- [ ] Gérer le cache, les versions, les chemins locaux et la récupération après téléchargement interrompu.
- [ ] Exposer l'état téléchargé, manquant, invalide ou en cours de mise à jour dans les Settings.
- [ ] Rendre le téléchargement automatique configurable sans désactiver silencieusement la protection API.
- [ ] Refuser la redaction API lorsque le modèle NER requis est absent ou invalide.
- [ ] Vérifier la compatibilité des anciens réglages et chemins persistés.
