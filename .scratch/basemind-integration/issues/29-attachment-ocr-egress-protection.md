# 29: Protection des pièces jointes et contenus visuels

**What to build:** Les pièces jointes qui accompagnent un tour suivent la même politique PII que le prompt, afin qu'un document ou une image ne puisse pas contourner la pseudonymisation API.

**Blocked by:** 25 (Seam commun de sortie vers le modèle), 28 (Redaction fail-closed des documents, fichiers et résultats RAG)

**Status:** ready-for-agent

- [ ] Redacter les pièces jointes texte avant tout envoi à un modèle API.
- [ ] Ajouter un chemin OCR/redaction vérifiable pour les images contenant du texte.
- [ ] Refuser une image ou pièce jointe API lorsqu'aucune transformation sûre n'est disponible.
- [ ] Conserver les pièces jointes originales pour un modèle local selon la route déclarée.
- [ ] Couvrir les data URLs, chemins de fichiers, presse-papiers et attachments distants.
- [ ] Vérifier qu'aucun payload brut n'est conservé dans les métadonnées du tour modèle.
