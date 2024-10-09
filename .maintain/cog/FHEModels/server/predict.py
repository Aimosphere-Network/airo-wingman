import os
from cog import BasePredictor, Input
from concrete.ml.deployment import FHEModelServer

# Suppose client has already interacted with dev to get server.zip 
# In real world this file would be copied to server directory once before
server_fhe_file_dir = "../fhe"


class Predictor(BasePredictor):
    def setup(self):
        self.fhe_circuit = FHEModelServer(server_fhe_file_dir)
    

    def predict(self, 
                eval_key_file: str = Input(description="serialized evaluation key"),
                enc_input_file: str = Input(description="serialized and quatized encrypted input"),
                ) -> str:

        # Get client's evaluation (public) key
        # TODO: find a way to have a separate COG function where we get eval key so each client
        # would only need to send eval key once
        with open(eval_key_file, "rb") as f:
             eval_keys = f.read()
        
        # Get encrypted input
        with open(enc_input_file, "rb") as f:
             enc_inputs = f.read()
       
        # Run circuit for each input
        enc_outputs = []
        for e_i in enc_inputs:
            e_o = self.fhe_circuit.run(e_i, eval_keys)
            enc_outputs.append(e_o)

        # Export encrypted results
        base_name, ext = os.path.splitext(enc_input_file) 
        enc_output_file = f"{base_name}{"_out"}{ext}"
        with open(enc_output_file, "wb") as f: f.write(enc_outputs)


