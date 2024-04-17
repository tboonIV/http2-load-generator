# HTTP2 Load Generator

## Motivation
The goal is to conduct a performance test on an HTTP/2 server using a small pool of connections but finding a suitable tool was challenging. Most tools are designed for testing web servers. As a result, many are blocking, forcing the use of additional connections to achieve higher throughput, and often the load generator consumes more CPU than my target server. 

This load generator is built to address these problems. It is optimized for testing backend HTTP/2 servers that communicate with other backend components with a fixed, long-living HTTP/2 connection.

## Usage:

Example to establish 4 HTTP/2 connections, each with 8000 TPS for a duration of 300 seconds.

```yaml
log_level: "Info"
parallel: 4
runner:
  target_rps: 8000
  duration: "300s"
  batch_size: "Auto"
  base_url: "http://localhost:8080"
  global:
    variables:
      - name: COUNTER
        function:
          type: Incremental
          start: 0
          threshold: 100000
          steps: 1
      - name: RANDOM
        function:
          type: Random
          min: 0
          max: 100000
  scenarios:
    - name: createSubscriber
      request:
        method: POST
        path: "/rsgateway/data/json/subscriber"
        body: |
          {
            "$": "MtxRequestSubscriberCreate",
            "Name": "James Bond",
            "FirstName": "James_${COUNTER}_${RANDOM}",
            "LastName": "Bond",
            "ContactEmail": "james.bond@email.com"
          }
      response:
        assert:
          status: 200
```