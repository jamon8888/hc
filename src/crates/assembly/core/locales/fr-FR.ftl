# OpenBitFun French Language Pack
# French (fr-FR) Fluent Translation File

# ==================== General ====================
app-version = Version { $version }
loading = Chargement...
welcome = Bienvenue dans Alex

# ==================== Actions ====================
action-confirm = Confirmer
action-cancel = Annuler
action-save = Enregistrer
action-delete = Supprimer
action-edit = Modifier
action-create = Créer
action-add = Ajouter
action-remove = Retirer
action-close = Fermer
action-open = Ouvrir
action-copy = Copier
action-paste = Coller
action-undo = Annuler
action-redo = Rétablir
action-refresh = Actualiser
action-search = Rechercher
action-retry = Réessayer
action-stop = Arrêter
action-start = Démarrer

# ==================== Status ====================
status-loading = Chargement
status-saving = Enregistrement
status-saved = Enregistré
status-success = Succès
status-error = Erreur
status-warning = Avertissement
status-info = Information
status-pending = En attente
status-processing = Traitement
status-completed = Terminé
status-failed = Échec
status-cancelled = Annulé
status-ready = Prêt
status-connected = Connecté
status-disconnected = Déconnecté

# ==================== File ====================
file-not-found = Fichier introuvable : { $path }
file-read-error = Échec de la lecture du fichier : { $path }
file-write-error = Échec de l'écriture du fichier : { $path }
file-delete-error = Échec de la suppression du fichier : { $path }
file-permission-denied = Permission refusée : { $path }
file-already-exists = Le fichier existe déjà : { $path }
file-saved = Fichier enregistré : { $path }
file-created = Fichier créé : { $path }
file-deleted = Fichier supprimé : { $path }

# ==================== Workspace ====================
workspace-opened = Espace de travail ouvert : { $path }
workspace-closed = Espace de travail fermé
workspace-not-found = Espace de travail introuvable
workspace-open-error = Échec de l'ouverture de l'espace de travail

# ==================== Git ====================
git-not-repository = Le répertoire courant n'est pas un dépôt Git
git-commit-success = Commit réussi
git-push-success = Push réussi
git-pull-success = Pull réussi
git-clone-error = Échec du clonage du dépôt
git-commit-error = Échec du commit
git-push-error = Échec du push
git-pull-error = Échec du pull
git-merge-conflict = Conflit de fusion existant
git-branch-created = Branche créée : { $name }
git-branch-deleted = Branche supprimée : { $name }
git-checkout-success = Basculé vers la branche : { $name }

# ==================== AI ====================
ai-connection-error = Échec de la connexion au service IA
ai-api-key-invalid = Clé API invalide
ai-model-not-found = Modèle introuvable : { $model }
ai-context-too-long = Contexte dépassant la limite
ai-rate-limited = Limite de taux dépassée
ai-generation-error = Échec de la génération de contenu
ai-thinking = Réflexion...
ai-generating = Génération...

# ==================== Terminal ====================
terminal-created = Terminal créé
terminal-closed = Terminal fermé
terminal-create-error = Échec de la création du terminal
terminal-command-error = Échec de l'exécution de la commande
terminal-shell-not-found = Shell introuvable

# ==================== Config ====================
config-loaded = Configuration chargée
config-saved = Configuration enregistrée
config-load-error = Échec du chargement de la configuration
config-save-error = Échec de l'enregistrement de la configuration
config-invalid = Format de configuration invalide
config-reset = Configuration réinitialisée

# ==================== Snapshot ====================
snapshot-created = Instantané créé : { $name }
snapshot-restored = Instantané restauré : { $name }
snapshot-deleted = Instantané supprimé
snapshot-create-error = Échec de la création de l'instantané
snapshot-restore-error = Échec de la restauration de l'instantané
snapshot-not-found = Instantané introuvable

# ==================== I18n ====================
language-changed = Langue changée pour : { $language }
language-not-supported = Langue non prise en charge : { $language }

# ==================== Notifications ====================
notification-copied = Copié dans le presse-papiers
notification-settings-saved = Paramètres enregistrés
notification-connection-established = Connexion établie
notification-connection-lost = Connexion perdue

# ==================== Errors ====================
error-unknown = Une erreur inconnue s'est produite
error-network = Erreur réseau
error-timeout = Délai d'attente dépassé
error-server = Erreur serveur
error-unauthorized = Non autorisé
error-forbidden = Accès refusé

# ==================== Time ====================
time-just-now = à l'instant
time-seconds-ago = il y a { $count } { $count ->
    [one] seconde
   *[other] secondes
}
time-minutes-ago = il y a { $count } { $count ->
    [one] minute
   *[other] minutes
}
time-hours-ago = il y a { $count } { $count ->
    [one] heure
   *[other] heures
}
time-days-ago = il y a { $count } { $count ->
    [one] jour
   *[other] jours
}
time-weeks-ago = il y a { $count } { $count ->
    [one] semaine
   *[other] semaines
}
time-months-ago = il y a { $count } { $count ->
    [one] mois
   *[other] mois
}
time-years-ago = il y a { $count } { $count ->
    [one] an
   *[other] ans
}