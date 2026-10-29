# Alex — L'IA des professionnels du droit des affaires

**Alex est un produit [Hacienda](#).**

**Alternative à Claude ou ChatGPT, Alex est l'assistant IA des avocats d'affaires, juristes d'entreprise et directions juridiques : vos dossiers restent sous votre contrôle, vos réponses sont sourcées, et la confidentialité n'est pas une option.**

Un contrat cédé sur un assistant IA généraliste, c'est une pièce confidentielle confiée à des serveurs que vous ne contrôlez pas, pour des finalités que vous ne décidez pas. Alex part du principe inverse : le dossier reste au cabinet, l'IA vient à lui.

---

## Pourquoi Alex plutôt qu'un assistant IA généraliste ?

| | Assistants IA grand public | Alex |
|---|---|---|
| **Localisation des données** | Hébergement étranger, juridiction non maîtrisée | Souveraine : Europe, cloud privé, ou vos serveurs |
| **Entraînement sur vos données** | Exclu contractuellement en offre pro, mais flou en pratique | Jamais, par conception — aucun usage secondaire |
| **Contexte de l'affaire** | Vous recollez les pièces à chaque conversation | Chaque dossier est un espace de travail persistant et cloisonné |
| **Fiabilité juridique** | Réponse générale sans référence | Réponse reliée à vos pièces et vos modèles, avec citation de la source |
| **Traçabilité** | Historique de chat, difficile à auditer | Journal d'audit horodaté : qui a posé quelle question, sur quel dossier |
| **Conformité** | À évaluer au cas par cas | Pensée pour RGPD, secret professionnel et obligations déontologiques |
| **Mode déconnecté** | Non | Oui : déploiement sur site ou air-gap possible |

Les assistants généralistes sont d'excellents produits. Ils ne sont simplement pas conçus pour un cabinet qui traite des dossiers d'affaires, ni pour une organisation qui doit prouver où vont ses données.

## Conçu pour qui ?

- **Cabinets d'avocats d'affaires** — corporate, M&A, droit des contrats, contentieux commercial : analyse, relecture et synthèse sans sortir les pièces du cabinet.
- **Juristes d'entreprise et directions juridiques** — revue contractuelle, secrétariat juridique, pilotage des risques, avec piste d'audit pour vos contrôles internes.
- **Avocats indépendants et collaborateurs** — un espace par dossier, des brouillons appuyés sur les modèles du cabinet.
- **Legal ops et conformité** — bibliothèques de clauses, procédures, cartographie des engagements.

## Les trois piliers

### 1. Confidentialité par conception

- **Déployez où vous voulez** : cloud souverain européen, cloud privé, serveurs du cabinet, ou poste isolé sans accès Internet.
- **Aucune donnée n'est utilisée pour l'entraînement** — pas d'exception, pas de case à décocher, pas de clause conditionnelle.
- **Chiffrement de bout en bout** des dossiers au repos et en transit.
- **Cloisonnement par dossier** : une pièce déposée dans le dossier d'un client n'alimente jamais les réponses d'un autre.
- Option **modèle 100 % local** : aucune requête ne quitte votre infrastructure, y compris les métadonnées.
- **Conçu pour vos obligations déontologiques** : contrôle d'accès, journalisation et non-réutilisation pour préserver le secret professionnel et la confidentialité des correspondances.

### 2. Vos dossiers comme contexte

- **Un espace par dossier** : importez statuts, pactes d'associés, baux commerciaux, protocoles, correspondances, PV d'assemblée, pièces de procédure. Alex s'en souvient entre deux sessions.
- **Réponses sourcées** : chaque affirmation est reliée au document et à la page dont elle provient. Vous vérifiez, vous ne croyez pas sur parole.
- **Analyse contractuelle** : « quels contrats contiennent une clause de non-concurrence ? », « signale les clauses déséquilibrées de ce bail », « quels engagements expirent avant juin ? ».
- **Brouillons de travail** : courriers, notes de synthèse, projets d'actes et de conditions générales, générés à partir du dossier et de vos modèles.

### 3. Prêt pour vos contraintes réglementaires

- **RGPD** : minimisation des données, durées de conservation configurables, registre des traitements, DPA fournie.
- **Piste d'audit** : chaque question, réponse et accès document est journalisé et exportable.
- **Contrôle d'accès fin** : par dossier, par utilisateur, par rôle. Révocation immédiate.
- **Réversibilité totale** : export et suppression complète d'un dossier sur demande, sans dépendance propriétaire.
- **Zéro rétention côté fournisseur** en déploiement privé : Alex n'est pas un point de collecte.

## Cas d'usage

- **Due diligence** — synthèse des statuts, PV et contrats identifiés d'une cible ; extraction des clauses de changement de contrôle.
- **Relecture contractuelle** — points de risque, clauses manquantes, déséquilibres, cohérence avec la position de négociation du cabinet.
- **Secrétariat juridique** — préparation d'assemblées, convocations, procès-verbaux, suivis des formalités.
- **Contentieux commercial** — chronologie du dossier, synthèse des pièces, préparation des conclusions.

## Comment ça marche

1. **Créez un dossier** et déposez-y vos pièces (PDF, Word, e-mails, notes).
2. **Travaillez avec Alex** : questions, synthèses, relectures, brouillons — avec citations vers vos sources.
3. **Archivez ou supprimez** : le dossier vous appartient, y compris son historique.

## Démarrage

```bash
# Exemple de lancement local (à adapter à votre distribution)
docker run -d \
  --name alex \
  -p 8080:8080 \
  -v ./mes-dossiers:/data \
  hacienda/alex:latest
```

Puis ouvrez `http://localhost:8080`. L'assistant peut fonctionner avec un modèle local, sans aucune connexion externe.

<!-- À compléter : liens réels vers la documentation, le guide de déploiement et le support -->

## Feuille de route

- Connecteurs vers vos espaces documentaires existants (GED, serveurs de fichiers, e-mail)
- Bibliothèque de clauses et modèles d'actes propres au cabinet
- Chiffrement côté client avec clés détenues par le cabinet
- Modèles entraînés sur les usages du droit des affaires, sans données clients
- Certifications et attestations d'hébergement selon vos exigences

## Questions fréquentes

**Mes documents servent-ils à améliorer le produit ?**
Non. Aucune donnée client n'est utilisée pour l'entraînement, quelle que soit l'offre.

**Puis-je faire tourner Alex entièrement hors ligne ?**
Oui. En déploiement sur site avec modèle local, aucune donnée ne sort de votre infrastructure.

**Alex préserve-t-il le secret professionnel ?**
Alex est conçu pour vous y aider : données cloisonnées par dossier, chiffrées, jamais réutilisées, et journalisées. Vous gardez la maîtrise complète de leur localisation et de leur cycle de vie.

**Qui peut consulter un dossier ?**
Uniquement les personnes que vous y autorisez. Les dossiers sont cloisonnés et l'accès est journalisé.

**Que se passe-t-il si je résilie ?**
Vous exportez vos dossiers dans des formats ouverts, puis tout est supprimé sur confirmation.

---

---

# Alex — The AI for business and commercial law professionals

**Alex is a product of [Hacienda](#).**

**An alternative to Claude or ChatGPT, Alex is the AI assistant for business lawyers, in-house counsel, and legal departments: your case files stay under your control, your answers are sourced, and confidentiality is not an option.**

Handing a contract to a general-purpose AI assistant means trusting confidential documents to servers you don't control, for purposes you don't decide. Alex starts from the opposite premise: the file stays at the firm, the AI comes to it.

---

## Why Alex instead of a general-purpose assistant?

| | General-purpose AI assistants | Alex |
|---|---|---|
| **Data location** | Foreign hosting, uncontrolled jurisdiction | Sovereign: Europe, private cloud, or your own servers |
| **Training on your data** | Contractually excluded on pro plans, but unclear in practice | Never, by design — no secondary use |
| **Case context** | You re-upload documents into every conversation | Each case file is a persistent, siloed workspace |
| **Legal reliability** | Generic answer without references | Answer grounded in your documents and templates, with source citation |
| **Traceability** | Chat history, hard to audit | Timestamped audit log: who asked what, on which file |
| **Compliance** | Case-by-case assessment | Built for GDPR, professional secrecy, and deontological obligations |
| **Offline mode** | No | Yes: on-premise or air-gapped deployment |

General-purpose assistants are great products. They are simply not built for a firm handling business law matters, nor for an organization that must prove where its data goes.

## Who is it for?

- **Business law firms** — corporate, M&A, contracts, commercial litigation: analysis, review, and summaries without documents leaving the firm.
- **In-house counsel and legal departments** — contract review, entity management, risk oversight, with an audit trail for your internal controls.
- **Independent lawyers and associates** — one workspace per case, drafts grounded in the firm's templates.
- **Legal ops and compliance** — clause libraries, procedures, mapping of commitments.

## The three pillars

### 1. Confidentiality by design

- **Deploy where you want**: European sovereign cloud, private cloud, the firm's servers, or an isolated workstation with no Internet access.
- **No data is ever used for training** — no exceptions, no checkbox, no conditional clause.
- **End-to-end encryption** for files at rest and in transit.
- **Per-case silos**: a document added to one client file never feeds answers for another.
- **Fully local model option**: no request ever leaves your infrastructure, metadata included.
- **Built for your deontological obligations**: access control, logging, and no reuse to preserve professional secrecy and correspondence confidentiality.

### 2. Your case files as context

- **One workspace per case**: import articles of association, shareholders' agreements, commercial leases, protocols, correspondence, meeting minutes, litigation exhibits. Alex remembers between sessions.
- **Sourced answers**: every claim links back to the document and page it comes from. You verify, you don't take its word for it.
- **Contract analysis**: "which contracts contain a non-compete clause?", "flag the unbalanced clauses in this lease", "which commitments expire before June?".
- **Work drafts**: letters, memos, deed and terms-and-conditions drafts, generated from the file and your own templates.

### 3. Ready for your regulatory constraints

- **GDPR**: data minimization, configurable retention periods, processing records, DPA provided.
- **Audit trail**: every question, answer, and document access is logged and exportable.
- **Fine-grained access control**: per file, per user, per role. Immediate revocation.
- **Full reversibility**: export and complete deletion of any case file on request, no proprietary lock-in.
- **Zero provider-side retention** in private deployments: Alex is not a collection point.

## Use cases

- **Due diligence** — summarize the target's articles of association, minutes, and identified contracts; extract change-of-control clauses.
- **Contract review** — risk points, missing clauses, imbalances, consistency with the firm's negotiation position.
- **Entity management** — preparing meetings, convening notices, minutes, filing follow-ups.
- **Commercial litigation** — case chronology, exhibit summaries, pleading preparation.

## How it works

1. **Create a case file** and drop in your documents (PDF, Word, emails, notes).
2. **Work with Alex**: questions, summaries, reviews, drafts — with citations to your sources.
3. **Archive or delete**: the file belongs to you, history included.

## Getting started

```bash
# Example local launch (adapt to your distribution)
docker run -d \
  --name alex \
  -p 8080:8080 \
  -v ./my-case-files:/data \
  hacienda/alex:latest
```

Then open `http://localhost:8080`. The assistant can run on a local model with no external connection at all.

<!-- TODO: real links to docs, deployment guide, and support -->

## Roadmap

- Connectors to your existing document repositories (DMS, file servers, email)
- Firm-owned clause and deed template libraries
- Client-side encryption with keys held by the firm
- Models tuned on business law usage patterns, without customer data
- Certifications and hosting attestations per your requirements

## FAQ

**Are my documents used to improve the product?**
No. No customer data is used for training, on any plan.

**Can Alex run fully offline?**
Yes. On-premise deployment with a local model sends nothing outside your infrastructure.

**Does Alex preserve professional secrecy?**
Alex is built to help you do so: data siloed per case, encrypted, never reused, and logged. You keep full control over their location and lifecycle.

**Who can see a case file?**
Only the people you grant access to. Files are siloed and access is logged.

**What happens if I cancel?**
You export your files in open formats, then everything is deleted upon confirmation.

---

*Alex est un produit Hacienda — Alex is a Hacienda product.*
*Draft v2 — liens, tarifs et certifications à valider avant publication.*
# CI trigger
