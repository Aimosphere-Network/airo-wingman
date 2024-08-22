cog build -t decision-tree-model

docker run -d -p 5001:5000 --platform=linux/amd64 decision-tree-model

curl http://localhost:5000/predictions -X POST \
    -H 'Content-Type: application/json' \
    -d '{"input": {"pkl": Path}}'

cog predict -i request="concrete_model/test_data0_10.pkl"

cog predict -i request="sklearn_model/test_data0_10.pkl"