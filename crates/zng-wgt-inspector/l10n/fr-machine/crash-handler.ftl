## Gestionnaire de plantage (Debug Crash Handler)

window =
    .title = {$app} - L'application a planté

## Panneaux (Panels)

# save-copy-starting-name:
#     nom de fichier par défaut
minidump =
    .open-error = Échec de l'ouverture du minidump.
        {$error}
    .remove-error = Échec de la suppression du minidump.
        {$error}
    .save-copy-filter-name = Minidump
    .save-copy-starting-name = minidump
    .save-copy-title = Enregistrer une copie
    .save-error = Échec de l'enregistrement de la copie du minidump.
        {$error}
    .title = Minidump

panic =
    .title = Panique

stderr =
    .title = Stderr

stdout =
    .title = Stdout

summary =
    .text = Horodatage : {$timestamp}
        Code de sortie : {$exit_code}
        Signal : {$signal}
        Stderr : {$stderr_len} octets
        Stdout : {$stdout_len} octets
        Panique : {$is_panic}
        Minidump : {$minidump_path}
        
        Arguments : {$args}
        OS : {$os}
    .title = Résumé

widget =
    .title = Widget