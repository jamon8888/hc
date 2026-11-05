# Pseudonymisation réversible des PII à la frontière LLM/API

**Status:** ready-for-agent

## Problem Statement

Les utilisateurs travaillent sur un dossier ouvert qui peut contenir des noms,
adresses, coordonnées, identifiants, informations bancaires, secrets et autres
PII. Ils veulent conserver le contexte d'un dossier entre leurs recherches,
leurs demandes et leurs sessions, sans envoyer ces valeurs en clair aux modèles
hébergés par des fournisseurs externes.

L'intégration Basemind actuelle fournit déjà la détection, la redaction
documentaire, le RAG, l'identité stable, le vault de session et une interception
Desktop partielle. Elle n'est toutefois pas uniforme : plusieurs chemins CLI,
headless, serveur et dispatch construisent directement des requêtes de tour ;
les pièces jointes ne suivent pas systématiquement la même politique ; et la
garantie dépend encore d'un port global publié par le Desktop.

La politique produit demandée est la suivante :

- toute donnée envoyée à un modèle via une API distante doit être
  pseudonymisée avant le franchissement de la frontière modèle ;
- les tokens doivent rester cohérents pour préserver le raisonnement et le
  contexte utilisateur ;
- la réhydratation doit rester possible pour l'affichage humain autorisé ;
- un modèle local n'a pas besoin de pseudonymisation et doit pouvoir recevoir
  le contexte original ;
- un chemin dont le type de modèle est inconnu ne doit pas être traité comme
  local par défaut.

## Solution

Introduire une politique commune de sortie vers les modèles, appliquée au plus
haut niveau partagé par le runtime natif, les adaptateurs de fournisseurs API,
les clients ACP et les chemins CLI/headless.

La politique résout explicitement la route du modèle :

- `ApiModel` : détection Basemind obligatoire, remplacement par tokens de
  session, persistance dans le vault, puis envoi du texte pseudonymisé ;
- `LocalModel` : passage du texte original autorisé, sans pseudonymisation ;
- `UnknownModelRoute` : refus avant envoi jusqu'à résolution explicite de la
  capacité du modèle.

Le pipeline API est :

1. recevoir le texte et les contextes d'un tour dans le runtime ;
2. déterminer la capacité de confidentialité du modèle sélectionné ;
3. pour une API distante, appeler Basemind `redact_text` avec la politique NER
   obligatoire ;
4. transformer chaque valeur détectée en identité stable du workspace ;
5. créer un token opaque HMAC, propre à la session ;
6. stocker `token -> valeur originale` et le sel de session dans le vault
   chiffré ;
7. envoyer uniquement le contexte pseudonymisé au fournisseur API ;
8. garder les sources originales dans le dossier local et les vues humaines
   réhydratables derrière une autorisation UI ;
9. rétokeniser les résultats RAG, résultats d'outils et contenus documentaires
   avant leur exposition au modèle ;
10. réhydrater uniquement à la frontière de rendu humain.

Le même contrat doit être utilisé par Desktop, CLI interactif, ACP, serveur,
peer host et detached dispatch. Les hôtes qui ne peuvent pas assurer ce contrat
doivent annoncer un état explicitement unsupported ou refuser l'opération.

## User Stories

1. En tant qu'utilisateur d'Alex, je veux que mon nom soit remplacé par un token
   opaque avant tout envoi à une API LLM, afin que le fournisseur externe ne
   reçoive pas mon identité réelle.

2. En tant qu'utilisateur d'Alex, je veux que les adresses, emails, téléphones,
   IBAN, cartes, identifiants, secrets et autres PII soient traités par la même
   politique, afin de ne pas dépendre d'une liste de formats incomplète.

3. En tant qu'utilisateur d'Alex, je veux que deux occurrences de la même
   personne dans mon dossier utilisent le même token de session, afin que le
   modèle conserve la cohérence du contexte.

4. En tant qu'utilisateur d'Alex, je veux que le même token puisse être retrouvé
   dans plusieurs tours d'une session, afin de poursuivre une recherche sans
   réintroduire manuellement le contexte.

5. En tant qu'utilisateur d'Alex, je veux que deux sessions différentes ne
   permettent pas de corréler directement les tokens, afin de limiter les liens
   entre contextes de travail.

6. En tant qu'utilisateur d'Alex, je veux voir le texte original dans l'interface
   autorisée, afin que la confidentialité côté modèle ne dégrade pas mon travail.

7. En tant qu'utilisateur d'Alex, je veux pouvoir révéler temporairement une
   valeur PII depuis une vue humaine, afin de vérifier une source sans injecter
   cette valeur dans le transcript ou dans le contexte du modèle.

8. En tant qu'utilisateur d'Alex, je veux que la révélation soit limitée au
   workspace et à l'identité autorisée, afin qu'un token d'un autre dossier ne
   puisse pas être résolu.

9. En tant qu'utilisateur d'Alex, je veux retrouver les mêmes entités dans les
   résultats RAG et dans les documents ouverts, afin que les recherches restent
   compréhensibles sur l'ensemble du dossier.

10. En tant qu'utilisateur d'Alex, je veux rechercher un nom déjà pseudonymisé
    dans une demande suivante, afin que le runtime puisse résoudre le token
    localement sans demander au modèle de connaître la valeur originale.

11. En tant qu'utilisateur d'Alex, je veux que les chunks RAG soient inspectés
    une seconde fois avant de sortir du runtime, afin qu'une détection incomplète
    de l'index ne puisse pas exposer une PII au modèle.

12. En tant qu'utilisateur d'Alex, je veux que les documents PDF, Word, tableurs,
    emails, Markdown, code et formats texte suivent une politique cohérente,
    afin qu'un changement d'extension ne crée pas de contournement.

13. En tant qu'utilisateur d'Alex, je veux que les images ou pièces jointes
    contenant du texte PII soient OCRées et redacted avant un envoi API, afin
    qu'une pièce jointe ne contourne pas la protection du prompt.

14. En tant qu'utilisateur d'Alex, je veux qu'une pièce jointe impossible à
    protéger soit refusée pour un modèle API, afin de ne pas recevoir un faux
    succès accompagné d'une fuite de données.

15. En tant qu'utilisateur d'Alex, je veux que mon modèle local puisse recevoir
    le texte original sans pseudonymisation, afin d'éviter un coût et une perte
    de lisibilité inutiles lorsque les données ne quittent pas ma machine.

16. En tant qu'utilisateur d'Alex, je veux que le choix local/API soit fondé sur
    une capacité déclarée du modèle et non sur une simple heuristique d'URL, afin
    qu'un proxy ou un endpoint local ne soit pas classé incorrectement.

17. En tant qu'utilisateur d'Alex, je veux que le runtime refuse un modèle dont
    la route de confidentialité est inconnue, afin qu'une nouvelle intégration
    ne contourne pas accidentellement la politique.

18. En tant qu'utilisateur du CLI Alex, je veux bénéficier de la même
    pseudonymisation qu'avec le Desktop lorsque j'utilise une API LLM, afin que
    changer de surface ne change pas mon niveau de protection.

19. En tant qu'utilisateur du CLI Alex, je veux que les jobs detached et les
    follow-up turns réutilisent le même vault de session, afin que le contexte
    reste cohérent après une déconnexion.

20. En tant qu'utilisateur d'un client ACP piloté par Alex, je veux que le prompt
    transmis à l'agent externe soit pseudonymisé avant de quitter Alex, afin que
    le modèle utilisé par cet agent ne voie pas les valeurs originales.

21. En tant qu'utilisateur mobile ou distant, je veux que le host qui possède le
    workspace applique la politique, afin que le contrôleur ne lise ni ne
    réhydrate par erreur un dossier local différent.

22. En tant qu'utilisateur d'un workspace SSH, je veux que Basemind s'exécute
    sur l'hôte qui possède les fichiers, afin que la redaction ne dépende pas du
    filesystem du contrôleur.

23. En tant qu'utilisateur d'Alex, je veux une erreur explicite si Basemind est
    indisponible pour une requête API, afin de savoir que le tour n'a pas été
    envoyé en clair.

24. En tant qu'utilisateur d'Alex, je veux que les erreurs de couverture NER,
    de vault ou de transport empêchent l'envoi API, afin qu'une redaction
    partielle ne soit jamais présentée comme complète.

25. En tant qu'administrateur, je veux que les anciens vaults et sessions restent
    lisibles après une mise à jour, afin de préserver la continuité des dossiers.

26. En tant qu'administrateur, je veux que les anciens clients distants puissent
    continuer à fonctionner avec une capacité négociée, afin qu'une mise à jour
    ne transforme pas silencieusement un peer compatible en fuite de PII.

27. En tant qu'auditeur, je veux pouvoir démontrer quel chemin a été appliqué à
    chaque tour, afin de distinguer API pseudonymisée, modèle local et refus.

28. En tant qu'équipe produit, je veux conserver la logique PII dans un service
    Basemind indépendant des surfaces, afin que Desktop, CLI et serveur partagent
    les mêmes invariants.

29. En tant qu'équipe produit, je veux que la réhydratation ne soit jamais
    exposée comme un outil agent, afin qu'un modèle ne puisse pas s'auto-révéler
    les valeurs originales.

30. En tant qu'équipe sécurité, je veux que l'absence d'un adaptateur de sortie
    soit détectée par les tests de fermeture, afin qu'un nouveau chemin de
    soumission ne puisse pas être ajouté sans politique PII.

## Implementation Decisions

- Le vocabulaire de domaine reste : redaction pour la transformation d'un
  contenu, pseudonymisation pour le remplacement réversible de valeurs,
  réhydratation pour l'affichage humain, identité d'entité pour la cohérence
  entre documents et tokens de session pour la représentation modèle.

- Cette spec remplace la décision antérieure selon laquelle les modèles locaux
  étaient aussi pseudonymisés. La nouvelle règle produit est API obligatoire,
  local exempté, route inconnue refusée. L'ADR de l'identité PII devra être
  amendée avant fusion de l'implémentation.

- Le seam principal est un service de politique de sortie modèle, placé au plus
  haut niveau commun avant toute sérialisation vers un provider LLM. Les
  adaptateurs natifs et ACP doivent l'utiliser ; les routes UI, CLI et dispatch
  ne doivent plus appeler directement une pseudonymisation ou envoyer un
  `AgentDialogTurnRequest` brut.

- La politique de sortie reçoit le modèle sélectionné, son profil de transport,
  la session, le workspace et le contexte sortant. Elle produit soit un payload
  pseudonymisé, soit un payload local original, soit un refus explicite.

- Le modèle ne doit pas être classé local uniquement parce que son endpoint est
  `localhost`, `127.0.0.1` ou un nom connu. La capacité doit être déclarée par
  la configuration du provider ou négociée par l'adaptateur.

- La détection API utilise Basemind `redact_text` avec NER obligatoire et
  refus si la couverture NER est tronquée ou invérifiable.

- L'identity registry reste par workspace et mappe la valeur vers une identité
  stable. Le session vault reste chiffré, contient le sel de session et la map
  token-vers-valeur, et utilise la clé du trousseau OS.

- Le token modèle reste opaque : catégorie + HMAC tronqué du sel de session et
  de l'identité stable. L'alias et le sel ne doivent pas être déductibles du
  token ; l'inversion passe uniquement par le vault.

- Les transcripts, métadonnées et payloads conservés pour le runtime ne doivent
  pas recevoir la map de réhydratation. Le texte original est réservé à la
  frontière d'affichage autorisée.

- Le RAG conserve son index dans la frontière de confiance locale. Les queries
  peuvent être résolues localement via le vault ; les hits doivent être
  re-tokenisés avant d'être retournés au modèle API.

- Les résultats d'outils, les lectures de fichiers, les snippets, les sorties
  de recherche et les rappels de contexte sont soumis à la même politique de
  sortie. Il ne doit plus exister de fallback raw lorsque la route API exige la
  protection.

- Les pièces jointes texte doivent passer par le pipeline de redaction. Les
  pièces jointes visuelles doivent disposer d'un chemin OCR/redaction vérifié
  ou être refusées pour les modèles API. Les modèles locaux peuvent conserver
  les pièces jointes originales selon leur politique locale.

- La réhydratation reste une opération UI-only. Elle doit être temporaire,
  limitée au workspace et absente du markdown modèle, du transcript modèle et
  des outils agent.

- Le sidecar Basemind reste un processus séparé accessible par MCP loopback
  authentifié. Alex continue de posséder les ports de runtime et la logique
  d'identité ; Basemind ne devient pas une dépendance de la couche assembly.

- Le CLI interactif, l'ACP CLI, le serveur, le peer host et le detached dispatch
  doivent publier ou recevoir la même capacité de sortie modèle. Tant que cette
  intégration n'est pas disponible, ces surfaces doivent répondre unsupported
  pour une requête API plutôt que transmettre le prompt brut.

- Le host qui possède le workspace doit exécuter Basemind pour les workspaces
  distants. Un contrôleur ne doit jamais substituer son propre workspace à
  celui du peer.

- Les chemins de suppression, expiration et migration doivent traiter ensemble
  l'identity registry et les session vaults associés, sans supprimer un dossier
  parce qu'une ancienne entrée PII est illisible.

- Le téléchargement lazy et la configuration utilisateur des modèles sont
  séparés du contrat de confidentialité. L'absence d'un modèle Basemind requis
  doit produire un état non disponible et non une redaction dégradée.

- La journalisation doit rester en anglais et ne doit jamais écrire de valeurs
  originales, maps de réhydratation, prompts raw ou tokens de secret.

## Testing Decisions

- Les tests valident le comportement observable à la frontière modèle : le
  provider API reçoit-il uniquement des tokens ; le provider local reçoit-il le
  texte original ; une route inconnue est-elle refusée ; une erreur Basemind
  empêche-t-elle l'envoi ? Les tests ne doivent pas dépendre de détails internes
  de hashage autres que les propriétés contractuelles.

- Le seam principal reçoit des fakes de modèle API et local, un fake Basemind,
  un fake vault et un workspace temporaire. Il vérifie toute la matrice de
  décision sans lancer un fournisseur externe.

- Les tests du service Basemind couvrent la détection, la couverture NER,
  l'identité stable, les tokens par session, la persistance chiffrée, la reprise
  après redémarrage et l'absence de valeur originale dans les payloads modèle.

- Les tests runtime couvrent Desktop, CLI interactif, ACP, serveur, peer host et
  detached dispatch en vérifiant qu'aucun chemin ne contourne le seam de sortie.

- Les tests documentaires couvrent PDF, DOCX, tableur, texte, code, email,
  Markdown et formats non supportés. Un format non protégé doit être refusé,
  jamais renvoyé raw à un modèle API.

- Les tests RAG couvrent query tokenisée, résolution locale, recherche sur index
  raw, re-tokenisation des hits, détection secondaire d'un chunk et absence de
  l'alias interne dans le payload agent.

- Les tests de pièces jointes couvrent texte, images avec OCR, images sans
  capacité OCR, chemins de fichiers et data URLs. Une pièce jointe non sûre doit
  bloquer uniquement la route API concernée.

- Les tests UI couvrent la vue source/redacted, le compte d'entités, la révélation
  temporaire, la fermeture du reveal, l'absence de la valeur dans l'historique
  modèle et la reprise après fermeture/réouverture.

- Les tests remote couvrent workspace local, workspace SSH, remote control, peer
  device et detached dispatch. Ils vérifient que le sidecar est exécuté sur le
  host propriétaire et qu'une capacité absente est signalée explicitement.

- Les tests de compatibilité couvrent les anciens payloads, sessions sans champ
  de capacité, anciens vaults, anciennes formes de tokens encore supportées et
  négociation entre versions différentes.

- Les tests de fermeture vérifient qu'un nouveau constructeur de tour ou un
  nouvel adaptateur provider ne peut pas atteindre un modèle API sans publier
  une décision de confidentialité.

- Les tests doivent rester focalisés par crate et par seam. Les vérifications
  de frontières Cargo et de fermeture des surfaces sont ajoutées lorsque les
  dépendances ou les commandes distantes changent.

## Out of Scope

- Pseudonymiser les modèles explicitement locaux ; cette exemption est une
  décision produit de cette spec.

- Modifier le fork Basemind pour lui ajouter l'identity registry ou la logique
  de réhydratation ; ces responsabilités restent dans Alex.

- Corréler une même identité entre workspaces, comptes ou organisations.

- Envoyer la map de réhydratation à un fournisseur, à un agent externe ou à un
  outil agent.

- Garantir la confidentialité d'un provider API configuré comme local sans
  capacité de transport déclarée ; cette configuration doit être refusée ou
  corrigée par l'administrateur.

- Remplacer les contrôles d'accès workspace, le chiffrement général des dossiers,
  la rétention, le registre RGPD ou les politiques d'entraînement des fournisseurs.

- Fournir une anonymisation statistique irréversible pour l'export, l'analytics
  ou la télémétrie ; ce sujet nécessite une politique distincte.

## Further Notes

- L'état actuel possède une fondation solide côté Desktop : sidecar Basemind,
  `PromptPseudonymizationPort`, `DocumentRedactionPort`, `RagSearchPort`,
  identity registry, session vault et UI de reveal.

- La spec existante de l'intégration Basemind reste la référence opérationnelle
  pour les capacités déjà livrées, mais cette spec est l'autorité pour la
  frontière commune API/local et pour la couverture CLI/headless.

- La mise en production ne doit pas être annoncée comme “complète” tant que les
  chemins CLI/API, les pièces jointes, le refus fail-closed des lectures raw et
  les scénarios distants ne sont pas couverts par des tests E2E.

- La gestion actuelle des modèles est principalement une préparation et un
  bundling de ressources. Le téléchargement lazy et les réglages de modèle
  doivent être traités comme un chantier séparé, sans affaiblir la politique de
  refus lorsque le modèle NER requis manque.

- Le choix API-only inverse la décision historique D2 de l'effort PII-safe RAG.
  Avant implémentation, il faut mettre à jour l'ADR et les tickets qui déclarent
  encore que les modèles locaux sont pseudonymisés.

- Le workspace contenait déjà des modifications non liées au moment de la
  rédaction. Cette publication n'en modifie aucune.
