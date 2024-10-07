import joblib
import os
import subprocess
from cog import BasePredictor, Input
from concrete.ml.deployment import FHEModelClient

# Abstraction of message passing
client_fhe_file_dir = "../fhe"

fhe_files_dir = "../fhe"
keys_dir = "keys"
keys_file = "serialized_evaluation_keys.ekl"

enc_test_data_dir = "enc_test_data"

enc_output_file = "encrypted_prediction.enc"

class Predictor(BasePredictor):
    def setup(self):
        self.client = FHEModelClient(client_fhe_file_dir)
    

    def predict(self, 
                input_file: str = Input(description="plain-text features file"),
                ) -> str:

        # Create private and evaluation keys
        eval_key = self.client.get_serialized_evaluation_keys()

        # Export keys
        with open(keys_dir + keys_file, "wb") as f: f.write(eval_key)

        # Load features from .csv file
        input_data = joblib.load(input_file)

        # Encrypt features
        enc_inputs = []
        for i in input_data:
            e_i = self.client.quantize_encrypt_serialize(i.reshape(1, -1))
            enc_inputs.append(e_i)

        # Export encrypted features
        enc_inputs_file = os.path.join(enc_test_data_dir, input_file)
        if not os.path.exists(enc_test_data_dir): 
            os.mkdir(enc_test_data_dir)
        joblib.dump(enc_inputs, enc_inputs_file)

        # Call cog 
        call = subprocess.run(['cog', 'predict', 
                            '-i', f'eval_key_dir=@../client/{keys_dir}/{keys_file}', 
                            '-i', f'enc_input_file=@../client/{enc_inputs_file}'], 
                            capture_output=True, text=True)
        print(call)

        # Decrypt results
        enc_output = joblib.load(enc_output_file)

        output_data = []
        for e_o in enc_output:
            o = np.argmax(client.deserialize_decrypt_dequantize(e_o), axis=1)
            output_data.append(o)

        print(output_data)
        print(joblib.load(os.path.join(ground_truths_dir, features)))