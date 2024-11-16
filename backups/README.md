> Сборка и запуск
- Необходим Rust & Cargo
- `cargo run`

> С Docker
- `docker build -t backups:latest .`
- `docker run -v <absolute path to src>:/src -v <absolute path to dst>:/dst -v <absolute path to config.yml>:/bin/config.yml -it backups:latest <command, optional>`

> С Docker Compose
- `docker built -t backups:latest .`
- `docker compose up`

Напоминание: для использования других команд, например `show-config`, надо вставить в `docker-compose.yaml` `command: ["backups", "show-config"]`. Также возможно создать пример конфиг-файла с помощью подкоманды `show-config --example -c <путь до файла>`.

Для docker-compose можно использовать переменные окружения:
 - `BACKUPS_CONFIG_PATH`
 - `BACKUPS_SRC_DIR`
 - `BACKUPS_DST_DIR`

Поддерживается `yml` и `json`, по умолчанию используется `yml`, определяется по расширению файла. Можно явно указать с помощью флага `-f <yml|yaml|json>`.

> Конфигурация
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
