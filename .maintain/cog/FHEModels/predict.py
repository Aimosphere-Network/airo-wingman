import joblib
import numpy as np
import os
import concrete.ml.sklearn as concrete
from cog import BasePredictor, Input, Path
from concrete.ml.deployment import FHEModelServer

# model_file = "model.pkl"
# train_data_file = "train_data.csv"

# Abstraction of message passing
server_fhe_file_dir = "fhe"

enc_output_file = "encrypted_prediction.enc"

class Predictor(BasePredictor):
    def setup(self):
        # Load sklearn model
        # sklearn_model = joblib.load(model_file)   
        # Load training data
        # train_data = joblib.load(train_data_file)
        
        # Convert model into concrete model
        # self.concrete_model = concrete.DecisionTreeClassifier.from_sklearn_model(sklearn_model, n_bits=6)

        # Generate circuit from trained model, using training data as shape
        #self.fhe_circuit = self.concrete_model.compile(train_data)
            
        # Generate keys for circuit, force new keygen everytime this runs
        #self.fhe_circuit.keygen(force=True)

        #self.fhe_circuit = joblib.load(circuit_file)
        #with open(circuit_file, "rb") as f:
        #    self.fhe_circuit = f.read()

        self.fhe_circuit = FHEModelServer(server_fhe_file_dir)
    

    def predict(self, 
                eval_key_file: str = Input(description="serialized evaluation key"),
                enc_input_file: str = Input(description="serialized and quatized encrypted input"),
                ) -> str:

        # Get client's evaluation (public) key
        # TODO: find a way to have a separate COG function where we get eval key because each client
        # would only need to send eval key once
        with open(eval_key_file, "rb") as f:
             eval_keys = f.read()
        
        # Get encrypted input
        with open(enc_input_file, "rb") as f:
             enc_inputs = f.read()
       
        enc_outputs = []
        for e_i in enc_inputs:
            # """Explicit FHE circuit run"""
            # # Quantize input (float)
            # q_i = self.concrete_model.quantize_input(i.reshape(1, -1))
                
            # # Encrypt input
            # q_i_enc = self.fhe_circuit.encrypt(q_i)

            # # Execute linear product in FHE (run circuit for prediction)
            # q_result_enc = self.fhe_circuit.run(q_i_enc)

            # # Decrypt result
            # q_result = self.fhe_circuit.decrypt(q_result_enc)

            # # De-quantize result
            # result = self.concrete_model.dequantize_output(q_result)

            # # Apply either the sigmoid if it is a binary classification task, which is the case in this 
            # # example, or a softmax function in order to get the probabilities (in the clear)
            # proba = self.concrete_model.post_processing(result)

            # # Since this model does classification, apply the argmax to get the class predictions (in the clear)
            # # mNote that regression models won't need the following line
            # result = np.argmax(proba, axis=1)

            # """Implicit FHE circuit run"""x
            # # result = self.concrete_model.predict(req, fhe="execute")

            e_o = self.fhe_circuit.run(e_i, eval_keys)
            enc_outputs.append(e_o)

        with open(enc_output_file, "wb") as f:
             f.write(enc_outputs)


