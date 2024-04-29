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
        value: 0
        function:
          type: Increment
          start: 0
          threshold: 100000
          step: 1
      - name: RANDOM
        value: 0
        function:
          type: Random
          min: 0
          max: 100000
  scenarios:
    - name: chargingDataCreate
      request:
        method: POST
        path: "/nchf-convergedcharging/v2/chargingdata"
        headers: 
        - content-type: "application/json"
        body: |
          {
            "notifyUri": "http://chf/callback/notify",
            "oneTimeEvent": true,
            "invocationSequenceNumber": ${COUNTER},
            "invocationTimeStamp": "2021-06-16T17:14:42.849Z",
            "nfConsumerIdentification": {
              "nFIPv6Address": "2001:db8:85a3::8a2e:370:7334",
              "nFIPv4Address": "198.51.100.1",
              "nFName": "046b6c7f-0b8a-43b9-b35d-6489e6daee91",
              "nodeFunctionality": "SMF",
              "nFPLMNID": {
                "mnc": "123",
                "mcc": "456"
              }
            }
          }
      response:
        assert:
          status: 201
        define:
          - name: chargingDataRef
            from: Header
            path: "location"
            function: 
              type: Split
              delimiter: "/"
              index: 3
    - name: chargingDataUpdate
      request:
        method: POST
        path: "/nchf-convergedcharging/v2/chargingdata/${chargingDataRef}/update"
        headers: 
        - content-type: "application/json"
        body: |
          {
            "invocationSequenceNumber": ${COUNTER},
            "invocationTimeStamp": "2021-06-16T17:14:42.849Z",
            "nfConsumerIdentification": {
              "nFIPv6Address": "2001:db8:85a3::8a2e:370:7334",
              "nFIPv4Address": "198.51.100.1",
              "nFName": "046b6c7f-0b8a-43b9-b35d-6489e6daee91",
              "nodeFunctionality": "SMF",
              "nFPLMNID": {
                "mnc": "123",
                "mcc": "456"
              }
            }
          }
      response:
        assert:
          status: 200
    - name: chargingDataRelease
      request:
        method: POST
        path: "/nchf-convergedcharging/v2/chargingdata/${chargingDataRef}/release"
        headers: 
        - content-type: "application/json"
        body: |
          {
            "invocationSequenceNumber": ${COUNTER},
            "invocationTimeStamp": "2021-06-16T17:14:42.849Z",
            "nfConsumerIdentification": {
              "nFIPv6Address": "2001:db8:85a3::8a2e:370:7334",
              "nFIPv4Address": "198.51.100.1",
              "nFName": "046b6c7f-0b8a-43b9-b35d-6489e6daee91",
              "nodeFunctionality": "SMF",
              "nFPLMNID": {
                "mnc": "123",
                "mcc": "456"
              }
            }
          }
      response:
        assert:
          status: 204
```