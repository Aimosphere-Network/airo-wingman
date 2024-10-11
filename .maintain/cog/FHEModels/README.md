### TO SIMULATE THIS LOCALLY ###
docker network create aimosphere_simulator


### DEVELOPER ###

# To generate developer files:
# server.zip is sent to infrastructure provider (server) to run predictions
# client.zip is sent to infrastructure provider (server) to be sent to client, so they can generate inference keys
python dev.py


### INFRASTRUCTURE PROVIDER  (SERVER) ###

cd server

# Build docker image 
cog build -t spam-detector-server

# Run docker image
# If your model uses a CPU:
docker run -d --network aimosphere-simulator -p 5000:5000 spam-detector-server
# If your model uses a GPU:
docker run -d --network aimosphere-simulator -p 5000:5000 --gpus all spam-detector-server
# If you're on an M1 Mac:
docker run -d --network aimosphere-simulator -p 5001:5000 --platform=linux/amd64 spam-detector-server

# Run inference when cog is in an M1 Mac:
curl http://localhost:5001/predictions -i -X POST \
    -H 'Content-Type: application/json' \
    -d '{"input": {"eval_key_file": "serialized_evaluation_keys.ekl",
    "enc_input_file": "test_data/0-10.enc"}}'

## Run inference using cog predict
cog predict -i eval_key_file=@serialized_evaluation_keys.ekl -i enc_input_file=@test_data/0-10.enc


### CLIENT ###

cd client

# Build docker image 
docker build -t spam-detector-client

# Run docker image
# If your model uses a CPU:
docker run -d --network aimosphere-simulator -p 5000:5000 spam-detector-client
# If your model uses a GPU:
docker run -d --network aimosphere-simulator -p 5000:5000 --gpus all spam-detector-client
# If you're on an M1 Mac:
docker run -d --network aimosphere-simulator -p 5001:5000 --platform=linux/amd64 spam-detector-client

# Run client:
python3 client.py -i test_data/0-10.csv -f encrypt

python3 client.py -i test_data/0-10.enc -f decrypt


_____________________________________________________________________________________________________________
helper:
    docker rmi -f $(docker images -a -q)

    cd client/legacy
    python3 test.py > test_results.txt