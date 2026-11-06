# 30: Basemind sur les hosts distants et headless

**What to build:** Pour un workspace distant, peer ou detached, Basemind s'exécute sur le host qui possède les fichiers et expose une capacité négociée au contrôleur.

**Blocked by:** 26 (Pseudonymisation ACP pour les agents externes), 27 (Fermeture CLI, serveur et detached dispatch), 28 (Redaction fail-closed des documents, fichiers et résultats RAG)

**Status:** ready-for-agent

- [ ] Provisionner ou vérifier le sidecar Basemind sur le host propriétaire du workspace.
- [ ] Définir la négociation de version, binaire, modèle NER et capacités de confidentialité.
- [ ] Projeter les opérations peer, remote control et detached dispatch sans substituer le workspace du contrôleur.
- [ ] Réutiliser le vault et l'identity registry sur le host qui exécute réellement le tour.
- [ ] Refuser explicitement une opération lorsque le host distant ne peut pas fournir Basemind.
- [ ] Tester reconnexion, replay par curseur et reprise après perte du contrôleur.
