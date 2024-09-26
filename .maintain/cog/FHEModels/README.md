cog build -t spam-detector


# If your model uses a CPU:
docker run -d -p 5000:5000 spam-detector
# If your model uses a GPU:
docker run -d -p 5000:5000 --gpus all spam-detector

curl http://localhost:5000/predictions -i -X POST \
    -H 'Content-Type: application/json' \
    -d '{"input": {"path": "./test_data/0-10.csv"}}'

    
# If you're on an M1 Mac:
docker run -d -p 5001:5000 --platform=linux/amd64 spam-detector

# Call cog:
## Format 01: works more often with docker
curl http://localhost:5001/predictions -i -X POST \
    -H 'Content-Type: application/json' \
    -d '{"input": {"eval_key_file": "keys/serialized_evaluation_keys.ekl",
    "enc_input_file": "enc_test_data/0-10.csv"}}'
## Formar 02: more suscint, runs setup everytime
cog predict -i eval_key_file=@keys/serialized_evaluation_keys.ekl -i enc_input_file=@enc_test_data/0-10.csv

# Test file that tests the functions used in cog outside the container
python3 test.py >> test_results.txt 2>&1

# Client simulator
python3 client.py 0-10.csv



docker rmi -f $(docker images -a -q)