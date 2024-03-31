# HTTP2 Load Generator

## Motivation
The goal is to conduct a performance test on an HTTP/2 server using a small pool of connections but finding a suitable tool was challenging. Most tools are designed for testing web servers. As a result, many are blocking, forcing the use of additional connections to achieve higher throughput, and often the load generator consumes more CPU than my target server. 

This load generator is built to address these problems. It is optimized for testing backend HTTP/2 servers that communicate with other backend components with a fixed, long-living HTTP/2 connection.