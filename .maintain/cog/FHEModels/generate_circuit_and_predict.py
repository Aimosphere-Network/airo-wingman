import joblib
import numpy as np
import concrete.ml.sklearn as concrete
import csv

model_file = "model.pkl"
train_data_file = "train_data.csv"
path = "test_data/test_data0-10.csv"
gt_data_file = "ground_truths/ground_truth0-10.csv"

ground_truths = joblib.load(gt_data_file)

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
for req, gt in zip(requests, ground_truths):
    """Explicit FHE circuit run"""
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

    # Apply either the sigmoid if it is a binary classification task, which is the case in this 
    # example, or a softmax function in order to get the probabilities (in the clear)
    proba = concrete_model.post_processing(result)

    # Since this model does classification, apply the argmax to get the class predictions (in the clear)
    # Note that regression models won't need the following line
    result = np.argmax(proba, axis=1)

    """Implicit FHE circuit run"""
    result2 = concrete_model.predict(req, fhe="execute")
    
    print("Explicit FHE:", result[0])
    print("Implicit FHE:", result2[0])
    print("Ground truth", gt)
    print("-------------------------------")
    