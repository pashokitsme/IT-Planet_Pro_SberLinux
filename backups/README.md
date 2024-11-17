# Сборка и запуск
> Протестировано на macOS.

1. Необходим Rust & Cargo
2. `cargo build --release`

# С Docker
1. `docker build -t backups:latest .`
2. `docker run -v <absolute path to src>:/src -v <absolute path to dst>:/dst -v <absolute path to config.yml>:/bin/config.yml -it backups:latest <command, optional>`

# С Docker Compose
1. `docker built -t backups:latest .`
2. `docker compose up`

Для docker-compose можно использовать переменные окружения: `BACKUPS_CONFIG_PATH`, `BACKUPS_SRC_DIR`, `BACKUPS_DST_DIR`.


Напоминание: для использования других команд, например `show-config`, надо вставить в `docker-compose.yaml` `command: ["backups", "show-config"]`. Также возможно создать пример конфиг-файла с помощью подкоманды `show-config --example -c <путь до файла>`.

# При помощи докера под Linux
Это сбилдит бинарник под linux+glibc при помощи докера, но его можно будет использовать без него

```sh
docker build --output type=local,dest=. -f build.Dockerfile .
```

# Использование

Все команды и поддкоманды поддерживают флаг `-h` 

## Конфигурация
Поддерживается `yml` и `json`, по умолчанию используется `yml`, определяется по расширению файла. Можно явно указать с помощью флага `-f <yml|yaml|json>`.
```yaml
tasks:
  - src: /src
    dst: /dst
    on:
      trigger:
        type: schedule
        every:
          - 10 seconds
      strategy: incremental
  - src: /src
    dst: /dst2
    on:
      trigger:
        type: schedule
        every:
          - friday
      strategy: differential
```
