# Key-Value Raft Server

The following is an implementation of a distributed key-value store using rust, raft, and
tokio.

### Architecture

There is a client and server. These communicate using the public protobuf requests and
responses. There is just a single message type and it uses `oneof` to determine which
command is desired.

This is done with a framed codec against tcp.

The client will have a single ip address to connect to first, if there's time we could
have a command that will fetch the current peer list and it could use those to distribute
it's queries. As is, it will use tokio to do the request and response but will do so in a
blocking manner on the main thread.

The server is comprised of a few processes:

1. client listener
2. peer listener
3. database & raft

#### Client listener

The client listener will create a new connection struct for each connection that comes in.
A connection struct will handle the stream of public proto requests. For each request it
will translate it into a command and forward the command to the database's channel.

A connection contains:

* Framed stream
* Framed sink
* Database handle (a Sender channel wrapped in a newtype)
* Connection handle (a Receiver channel and Sender channel)

When the client has a request, it will generate a query for the DB and send it on the database
handle. The command will contain a connection handle so that the database can reply with a
response when it's ready.

The connection handle will deliver replies from the db to the connection. The replies
will be mapped to a proper protobuf response and then forwarded to the sink.

#### Peer listener

The peer listener will open up a listener on a different port and handle messages
delivered from the network. These don't have replies so it just forwards them on to the database
as raft messages.

#### Database & Raft

The main server is a database struct that holds the raft node and other state. The database
will have an in-memory representation of the data (if this OOM it dies, I'm not coding around
the size of memory). The raft log (the binary data) will be composed of protobuf data. Since I
only care about the state of the hash map, each normal entry will basically map to the basic
edit commands (set and delete) and be a protobuf. The snapshot will also be protobuf to
inflate the hashmap.

Each command will drive either a proposal from a connection (pushing a callback onto a lookup
table with the command id to handle when the command becomes committed. If it's just a read
command, we can just generate and send the callback immediately.

The read commands could be distributed to other threads if the state of the system is placed
behind a rwlock. This is an optimization we can handle later!

If a raft message is received it's driven into the raft node, then we go through the readiness
checks. Timeouts are done on the command channel.


#### Storage

While everything above can be handled with memory storage, the goal is also to persist changes
to disk. To do that we'll need to implement the storage trait for raft. We'll just write
things to various protobufs.

The storage module is written so that only upon snapshots are things considered persisted. This
guarantees that the data in a snapshot is what will be restored.

It uses the provided memory storage to manage the entries, instead of rebuilding it. However,
this is wrapped behind a layer that writes to disk. It's the snap shots that actually persist
and they will block the main execution loop.


## How to use

Clone the repo and build it to start. Then use the docker-compose file to launch and up the service:

```bash
docker-compose up --build
```

This will start 3 servers that are not connected at first. You can get them to connect by
connecting them in order. We need to get them to agree on the leader and that can
take some restarts to get correct. 2 nodes can end up going back and forth on the election.

To simplify it, there's a reset.sh script. Run this from the cli to build the docker cluster
and stand it up.

The trick appears to be to stand up one, get it to elect itself. This will allow you to propose
conf changes like adding a node. Add the node and it will try to start the election. However,
if both nodes think that they are the leader we get into a cycle which requires an interrupt
such one wins, that's what I'm using a restart for.

All commands can be done using the client binary, include `set`, `delete`, `scan`, `add_node`,
`remove_node`, `info`, and `ping`.

These were mapped to the CLI as well allowing you to interact from the CLI. `info` is very useful
for connecting new machines because it tells you the status of whatever node you are asking for.

When a node dies, it can rejoin and "catch up" in the logs. However, if it's very far behind, it
can be faster to remove it complete, kill it, and add it as a new node. This will allow it to
catch-up via a snap shot from another node instead.
