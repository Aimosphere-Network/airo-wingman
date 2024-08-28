import joblib
import numpy as np
import concrete.ml.sklearn as concrete

model_file = "model.pkl"
train_data_file = "train_data.csv"
path = "test_data/test_data0-10.csv"

 # Load model
sklearn_model = joblib.load(model_file)   
# Load training data
train_data = joblib.load(train_data_file)
        
# Convert model into concrete model
concrete_model = concrete.DecisionTreeClassifier.from_sklearn_model(sklearn_model, n_bits=6)
        
# Generate circuit from trained model, using training data as shape
fhe_circuit = concrete_model.compile(train_data)
            
# Generate keys for circuit, force new keygen everytime this runs
fhe_circuit.keygen(force=True)

# Load features from .csv file
requests = joblib.load(path)

# Predict for each request
for req in requests:
    # Quantize input (float)
    q_req = concrete_model.quantize_input(req.reshape(1, -1))
            
    # Encrypt the input
    q_req_enc = fhe_circuit.encrypt(q_req)

    # Execute the linear product in FHE (run circuit for prediction)
    q_result_enc = fhe_circuit.run(q_req_enc)

    # Decrypt result (integer)
    q_result = fhe_circuit.decrypt(q_result_enc)

    # De-quantize result
    result = concrete_model.dequantize_output(q_result)

    print(q_result.flatten().tolist())