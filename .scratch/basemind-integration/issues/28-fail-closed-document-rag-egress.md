# 28: Redaction fail-closed des documents, fichiers et résultats RAG

**What to build:** Aucun document, fichier, snippet, résultat d'outil ou résultat RAG contenant une PII ne peut atteindre un modèle API en clair, même lorsque Basemind est indisponible ou qu'un chemin de lecture inattendu est utilisé.

**Blocked by:** 24 (Contrat de route de confidentialité API/local), 25 (Seam commun de sortie vers le modèle)

**Status:** ready-for-agent

- [ ] Supprimer les retours raw implicites pour les lectures destinées à un modèle API.
- [ ] Refuser les documents lorsque le port de redaction n'est pas publié ou que la couverture NER est incertaine.
- [ ] Appliquer la même règle aux fichiers texte/code et aux fenêtres de lecture bornées.
- [ ] Conserver la vue source humaine sans l'utiliser comme source de contexte modèle non protégée.
- [ ] Garantir que les hits RAG sont re-tokenisés avant leur retour au modèle API.
- [ ] Vérifier que l'alias interne, la valeur originale et la map de réhydratation ne sortent jamais du runtime vers le modèle.
