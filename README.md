# Dodemansknop

"Dodemansknop" is 🇳🇱 Dutch 🇳🇱 for "dead man's switch". It is a simple tool to implement such a dead mans switch in your infrastructure with common tools like Prometheus and Alertmanager.

## Supported alerting targets

Currently, this supports the following alerting targets:

- Generic Webhooks
- Slack

Support for other targets is planned:

- OpsGenie
- PRs for other targets are welcome

## Usage

1. Provide a configuration file. See [config.example.yaml](config.example.yaml) for an example.

2. Run Dodemansknop with the configuration file as argument: `dodemansknop -config config.yaml`:

    ```
    $ docker run \
        -p 3030:3030 \
        -v $PWD/config.yaml:/config.yaml \
        ghcr.io/mittwald/dodemansknop:latest -config /config.yaml
    ```

3. Create a Prometheus Alert that continuously fires:

   ```yaml
   name: Dodemansknop
   expr: vector(1)
   labels:
     severity: none
   ```

4. Configure your Prometheus Alertmanager to route this alert to Dodemansknop:

    ```yaml
    route:
      ...
      routes:
      - match:
        alertname: Dodemansknop
        receiver: 'dodemansknop'
        group_wait: 0s
        group_interval: 1m
        repeat_interval: 50s
    receivers:
    - name: 'dodemansknop'
      webhook_configs:
      - url: 'http://dodemansknop/ping/service-id'
        send_resolved: false
          ``` 
    ```