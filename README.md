# JARL - Just A Rate Limiter

## What's JARL?

JARL provides an answer to the question, _how long do I need to wait until I can make another call to a given service in order to respect its rate limits?_ for distributed (service/micro-service) systems.

Instead of holding connections open or rejecting connections, this software calculates the amount of time to wait until a new request can be sent to a given external resource based on the number of requests already made within a specified period. Once a TCP socket is opened, it will return a string representation of a float as bytes with 3 decimal places.

JARL is service-agnostic and meant at enabling synchronization of distributed systems. It does not perform any requests to other endpoints. JARL does not attempt to distribute the requests evenly over the configured period (as there is no buffering of requests).


## Suggested Implementation

Run JARL on a host with enough available sockets to handle the maximum amount of simultaneous internal requests at any given time, logically mapping a specified port to the rate-limit of a target service.


### Example

A ficticious system needs to respect rate-limited calls to a payment gateway and a social media website for distinct workflows.

- Calls to the payment gateway may be made by a web codebase, plus several workers/tasks that handle different webhook messages. The payment gateway enforces a limit of 100 messages per second.

- The social media website's API will be used to purge OG tags from several registered URLs when descriptions and metadata change. It enforces a limit of 200 messages per minute.

Given that there are 5 workers handling the payment gateway-related messages and 50 for the social media, it is expected that a single Linux machine will be able to cope with all network and sockets required for all connections to be live at any given time. In this case, one instance of JARL would be run on one port to serve as the rate-limiter for the payment gateway, and another instance would be run on a different port to support requests for the social media website.

**It is expected that clients wait the number of seconds returned by JARL immediately after receiving the response from the service.**


## Running

There are no configuration files or environment variables required. There is also no output expected from the application in its host, except in the event of a panic. The executable is a CLI program with the following interface:

```
Usage: jarl --service <SERVICE> --period <PERIOD> --requests <REQUESTS> --ip <IP> --port <PORT>

Options:
      --service <SERVICE>    Name of the service to rate-limit, not used by the code, serving as a CLI reference only
      --requests <REQUESTS>  Maximum number of requests to allow within the period
      --period <PERIOD>      Period to enforce rate over, in seconds
      --ip <IP>              IPv4 interface to bind to, normally 0.0.0.0
      --port <PORT>          Port to bind to
  -h, --help                 Print help
```

The application does not currently supports signal handling.

In the examples given above, two instances of JARL would be created and set to run in the background, and later `disowned` (assuming no other background jobs are running):

```bash
$ jarl --service PaymentGateway --period 1 --requests 100 --ip 0.0.0.0 --port 1234 > payment_gateway.log &
$ jarl --service SocialMedia --period 60 --requests 200 --ip 0.0.0.0 --port 1235 > social_media.log &
$ disown %1 %2
```

### Clients

Clients should open a TCP socket to the host:port associated with the service to be called immediately before issuing the external/target API request. No data is expected to be sent to JARL. A minimum of 3 bytes will be returned by JARL (the string `0.0`) and a theoretical maximum of the string representation of the rate-limited period followed by two extra bytes (the string `.0`).

An example implementation of a Python client function:

```python
import socket
from time import sleep


def get_delay():
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
        # before connecting, it would be wise to settimeout on the socket 
        # object according to your network averages
        s.connect((HOST, PORT))
        data = s.recv(7)
        delay: float = float(data.decode())

        # sleep immediately before making the call to the rate-limited resource
        sleep(delay)
        
        # and then make the call
        ... work that API magic here ...
```

In the event JARL is unavailable or unresponsive, applications should fall back to their normal handling of exceeded target rate limits until JARL resumes normal operations.


## Performance

Non-scientific tests on a Windows machine using a debug build took a maximum of 1μs to calculate the delay float. The network latency against loopback was ~232μs.

JARL is not meant to be highly-available. It should also not be treated as a SPOF for an application, expecting to be ignored in the event of a malfunction.

## Building

The usual: `cargo build --release`


## How Does it Work?

A varying-size deque is created to hold a maximum of `|requests|`, and a TCP listener is bound to host:port as specified in the CLI. The server is single-threaded, and a lock is applied to the state represented by the deque as each connection is handled. A `base_delay` is calculated, which is equal to the expected amount of time each request to the final endpoint should take.

The deque is populated with the UNIX timestamps of the received requests until it adds `|requests|` elements. Until the deque reaches the number of requests specified as part of the rate limit, no other calculations are made and JARL returns `0.0`.

Once one item is added beyond the expected length of the deque, the last item is popped and the time delta to the first element is calculated.
- If the delta is **below** the rate limit period, that means that the requesting application needs to _wait_ before sending its request. The difference between the rate limit period and the calculated time delta is returned, _plus_ JARL's `base_delay` multiplied by the number of subsequent requests that would have exceeded the rate limit.
- If the delta is **above** the rate limit period, that means that the requests to the target endpoint will have taken longer than the minimum permissible time, and `0.0` is returned. This also sets the number of subsequent requests that would have exceeded the rate limit to 0.

The responsibility of delaying the requests to the target endpoint lies with the caller application, as does the responsibility of ignoring JARL in the event it becomes unresponsive or unavailable.


### Memory & Runtime Requirements

Most of the memory consumption will likely come from the deque. That should have a maximum equal to $\lceil{log _{2}n}\rceil *8$ bytes, assuming the deque will keep the underlying array at the closest power of 2 size. $n$ is the maximum number of requests defined by the target rate limit.

One should probably add the executable size to that.

There should be enough available sockets and file descriptors for all possible parallel connections.


## License

This project is licensed under the terms of the [MIT](LICENSE.md) license.
