# 27: Fermeture CLI, serveur et detached dispatch

**What to build:** Les surfaces CLI, serveur, peer host et detached dispatch appliquent la même politique API/local que Desktop et conservent le contexte pseudonymisé après déconnexion.

**Blocked by:** 24 (Contrat de route de confidentialité API/local), 25 (Seam commun de sortie vers le modèle)

**Status:** ready-for-agent

- [ ] Faire passer les tours CLI interactifs par la politique de sortie commune.
- [ ] Faire passer les tours peer host, follow-up turns et jobs detached par le même seam.
- [ ] Démarrer ou connecter Basemind dans les profils serveur et headless qui exécutent une API LLM.
- [ ] Réutiliser le session vault et l'identity registry du workspace cible.
- [ ] Refuser explicitement les jobs API lorsqu'aucun pipeline de pseudonymisation n'est disponible.
- [ ] Tester la reprise après déconnexion et le maintien des tokens entre deux workers.
