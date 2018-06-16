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
