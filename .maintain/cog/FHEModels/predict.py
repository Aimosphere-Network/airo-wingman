import joblib
import numpy as np
import concrete.ml.sklearn as concrete
from cog import BasePredictor, Input, Path

model_file = "model.pkl"
train_data_file = "train_data.csv"

class Predictor(BasePredictor):
    def setup(self):
        # Load sklearn model
        sklearn_model = joblib.load(model_file)   
        # Load training data
        train_data = joblib.load(train_data_file)
        
        # Convert model into concrete model
        self.concrete_model = concrete.DecisionTreeClassifier.from_sklearn_model(sklearn_model, n_bits=6)
        
        # Generate circuit from trained model, using training data as shape
        self.fhe_circuit = self.concrete_model.compile(train_data)
            
        # Generate keys for circuit, force new keygen everytime this runs
        self.fhe_circuit.keygen(force=True)

    def predict(
        self,
        path: Path = Input(description="csv file containing features"),
        ) -> str:

        # Load features from .csv file
        requests = joblib.load(path)
        
        results = []
        for req in requests:
            """Explicit FHE circuit run"""
            # Quantize input (float)
            q_req = self.concrete_model.quantize_input(req.reshape(1, -1))
            
            # Encrypt input
            q_req_enc = self.fhe_circuit.encrypt(q_req)

            # Execute linear product in FHE (run circuit for prediction)
            q_result_enc = self.fhe_circuit.run(q_req_enc)

            # Decrypt result
            q_result = self.fhe_circuit.decrypt(q_result_enc)

            # De-quantize result
            result = self.concrete_model.dequantize_output(q_result)

            # Apply either the sigmoid if it is a binary classification task, which is the case in this 
            # example, or a softmax function in order to get the probabilities (in the clear)
            proba = self.concrete_model.post_processing(result)

            # Since this model does classification, apply the argmax to get the class predictions (in the clear)
            # Note that regression models won't need the following line
            result = np.argmax(proba, axis=1)

            """Implicit FHE circuit run"""
            # result = self.concrete_model.predict(req, fhe="execute")

            results += list(result)
        
        print(" ".join(map(str, results))) 


