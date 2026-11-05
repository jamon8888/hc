# 32: Validation E2E, compatibilité et fermeture de sécurité

**What to build:** La politique API/local est démontrée de bout en bout sur les surfaces supportées et aucun nouveau chemin LLM ne peut contourner la pseudonymisation, la redaction ou les règles de capacité.

**Blocked by:** 25 (Seam commun de sortie vers le modèle), 26 (Pseudonymisation ACP pour les agents externes), 27 (Fermeture CLI, serveur et detached dispatch), 28 (Redaction fail-closed des documents, fichiers et résultats RAG), 29 (Protection des pièces jointes et contenus visuels), 30 (Basemind sur les hosts distants et headless), 31 (Cycle de vie des modèles Basemind et Settings)

**Status:** ready-for-agent

- [ ] Tester Desktop, CLI, ACP, serveur, peer et detached avec un faux provider API et un faux modèle local.
- [ ] Vérifier les propriétés de cohérence des tokens, de réhydratation UI et d'absence de PII dans les payloads modèle.
- [ ] Couvrir documents, fichiers, RAG, pièces jointes, OCR, erreurs Basemind et couverture NER tronquée.
- [ ] Couvrir workspace local, SSH, remote control, peer device et reprise après déconnexion.
- [ ] Ajouter les tests de legacy des vaults, sessions, configurations et payloads inter-version.
- [ ] Ajouter une fermeture automatisée empêchant l'ajout d'un chemin de soumission API sans décision de confidentialité.
- [ ] Produire les preuves de vérification ciblées et les déclarer dans la documentation de livraison.
