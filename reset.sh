set -x

# This script should get the cluster up and running.

docker-compose stop
docker-compose rm
docker-compose up -d --build

# sleep to allow it to boot
sleep 10;

# add node 2
./target/debug/client -h 0.0.0.0:19001 add_node 2 db2:9002
sleep 3;

# Restart to create a leaderless node 1
docker-compose restart db1

# Wait on db1. It should become a follower
sleep 3;

# Now we add db1 to db2 so db2 becomes a leader
./target/debug/client -h 0.0.0.0:19002 add_node 1 db1:9001

# Also add each other to get them in the db
sleep 3;
./target/debug/client -h 0.0.0.0:19002 add_node 2 db2:9002
sleep 3;
./target/debug/client -h 0.0.0.0:19001 add_node 1 db1:9001

# Now we can add db3 to db2 and db1
./target/debug/client -h 0.0.0.0:19002 add_node 3 db3:9003
./target/debug/client -h 0.0.0.0:19001 add_node 3 db3:9003

# Now we need db3 to know about the others. To do that we add one node
./target/debug/client -h 0.0.0.0:19003 add_node 1 db1:9001
docker-compose restart db3
# Wait on db3
sleep 3;
./target/debug/client -h 0.0.0.0:19003 add_node 2 db2:9002
