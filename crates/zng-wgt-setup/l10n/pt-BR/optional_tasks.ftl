additional_shortcuts =
    .info = { $tasks ->
        [one] Criar atalho adicional?
        *[other] Quais atalhos adicionais devem ser criados?
        }
    .title = { $tasks ->
        [one] Atalho Adicional
        *[other] Atalhos Adicionais
        }

# message:
#     Message only shows for $tasks > 1
associate_files =
    .info = { $tasks ->
        [one] Este tipo de arquivo deve abrir com {$app} por padrão?
        *[other] Quais tipos de arquivo devem abrir com {$app} por padrão?
        }
    .message = { $tasks ->
        *[other] Selecione os tipos de arquivo que você gostaria de associar ao {$app}.
        }
    .title = { $tasks ->
        [one] Associar Tipo de Arquivo
        *[other] Associar Tipos de Arquivo
        }

generic =
    .info = Quais tarefas adicionais devem ser executadas?
    .message = Selecione as tarefas adicionais que você gostaria de configurar para executar.
    .title = Selecionar Tarefas Adicionais

task-create-desktop-shortcut = Criar atalho na área de trabalho