### DEVELOPER ###

# To generate developer files:
# server.zip is sent to infrastructure provider (server) to run predictions
# client.zip is sent to infrastructure provider (server) to be sent to client, so they can generate inference keys
python dev.py


### INFRASTRUCTURE PROVIDER ###

# Build docker image 
cog build -t spam-detector-server

# Run docker image
# If your model uses a CPU:
docker run -d -p 5000:5000 spam-detector
# If your model uses a GPU:
docker run -d -p 5000:5000 --gpus all spam-detector
# If you're on an M1 Mac:
docker run -d -p 5000:5000 --platform=linux/amd64 spam-detector

### CLIENT ###

python3 client.py 0-10.csv

# In cog

# Build docker image 
cog build -t spam-detector-client

# Run docker image
# If your model uses a CPU:
docker run -d -p 5001:5000 spam-detector-client
# If your model uses a GPU:
docker run -d -p 5001:5000 --gpus all spam-detector-client
# If you're on an M1 Mac:
docker run -d -p 5001:5000 --platform=linux/amd64 spam-detector-client

# If you already have your input encrypted and inference keys generated, you can also:
# Run inference when cog uses CPU or GPU:
curl http://localhost:5000/predictions -i -X POST \
    -H 'Content-Type: application/json' \
    -d '{"input": {"path": "./test_data/0-10.csv"}}'
# Run inference when cog is in an M1 Mac:
curl http://localhost:5001/predictions -i -X POST \
    -H 'Content-Type: application/json' \
    -d '{"input": {"eval_key_file": "keys/serialized_evaluation_keys.ekl",
    "enc_input_file": "enc_test_data/0-10.csv"}}'
## Run inference using cog predicut
cog predict -i eval_key_file=@keys/serialized_evaluation_keys.ekl -i enc_input_file=@enc_test_data/0-10.csv


### AUX TEST ###

# Test file that tests the functions used in cog outside the container
python3 test.py >> test_results.txt 2>&1



_____________________________________________________________________________________________________________
docker rmi -f $(docker images -a -q)