# 26: Pseudonymisation ACP pour les agents externes

**What to build:** Les agents externes pilotés par ACP reçoivent des prompts pseudonymisés lorsqu'ils utilisent une API, avec le même contexte de session que le runtime natif.

**Blocked by:** 24 (Contrat de route de confidentialité API/local), 25 (Seam commun de sortie vers le modèle)

**Status:** ready-for-agent

- [ ] Brancher OpenCode, Claude Code, Codex et les autres clients ACP sur la politique de sortie commune.
- [ ] Conserver la continuité des tokens et du session vault entre les tours ACP.
- [ ] Autoriser le texte original uniquement lorsque l'ACP annonce explicitement un modèle local.
- [ ] Refuser l'envoi si le sidecar, le vault ou la capacité de modèle API est indisponible.
- [ ] Vérifier que le chemin CLI ACP bénéficie du même comportement que le chemin Desktop ACP.
