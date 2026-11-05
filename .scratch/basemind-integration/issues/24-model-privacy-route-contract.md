# 24: Contrat de route de confidentialité API/local

**What to build:** Le runtime connaît la route de confidentialité du modèle sélectionné et refuse toute route inconnue avant qu'un prompt ou contexte ne puisse sortir d'Alex.

**Blocked by:** None (can start immediately)

**Status:** ready-for-agent

- [ ] Définir une capacité de modèle distinguant explicitement `ApiModel`, `LocalModel` et `UnknownModelRoute`.
- [ ] Rendre cette capacité disponible aux providers natifs, adaptateurs ACP, CLI, serveur et dispatch.
- [ ] Faire refuser une route inconnue au lieu de la classer localement par heuristique d'URL.
- [ ] Préserver la lecture des configurations existantes avec une valeur par défaut compatible et sûre.
- [ ] Ajouter les tests de contrat de sérialisation, négociation et décision API/local.
