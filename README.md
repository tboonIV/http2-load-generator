# HTTP2 Load Generator

## Motivation
The goal is to conduct a performance test on an HTTP/2 server using a small pool of connections but finding a suitable tool was challenging. Most tools are designed for testing web servers. As a result, many are blocking, forcing the use of additional connections to achieve higher throughput, and often the load generator consumes more CPU than my target server. 

This load generator is built to address these problems. It is optimized for testing backend HTTP/2 servers that communicate with other backend components with a fixed, long-living HTTP/2 connection.

## Usage:

```bash
http2-load-generator --config ./config.yaml
```
The load generator is configured using a YAML file. Here is an example that configures the load generator to establish 4 HTTP/2 connections, each with 8000 TPS, for a total of 32000 TPS, and a duration of 300 seconds.

```yaml
log_level: "Info"
parallel: 4
runner:
  target_rps: 8000
  duration: "300s"
  batch_size: "Auto"
  base_url: "http://localhost:8080"
```

Full [config.yaml](./config.yaml)