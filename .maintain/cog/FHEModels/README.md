cog build -t decision-tree-model


# If your model uses a CPU:
docker run -d -p 5001:5000 my-model
# If your model uses a GPU:
docker run -d -p 5001:5000 --gpus all my-model
# If you're on an M1 Mac:
docker run -d -p 5001:5000 --platform=linux/amd64 my-model


curl http://localhost:5000/predictions -X POST \
    -H 'Content-Type: application/json' \
    -d '{"input": {"pkl": Path}}'

cog predict -i path=@test_data/test_data0-10.csv

python3 test.py >> test_results.txt 2>&1