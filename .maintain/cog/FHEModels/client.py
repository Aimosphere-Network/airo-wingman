import joblib
import os
import sys
import subprocess
import joblib
import numpy as np
from concrete.ml.deployment import FHEModelClient

test_data_dir = 'test_data'
enc_test_data_dir = 'enc_test_data'
ground_truths_dir = 'ground_truths'

fhe_files_dir = "fhe"
keys_dir = "keys"
keys_file = "serialized_evaluation_keys.ekl"
enc_output_file = "encrypted_prediction.enc"


client_file = sys.argv[1]

# Create client object to manage keys
client = FHEModelClient(fhe_files_dir, key_dir=keys_dir)

# Create private and evaluation keys
eval_key = client.get_serialized_evaluation_keys()

# Export keys
with open(keys_dir + keys_file, "wb") as f: f.write(eval_key)

# Load features from .csv file
input_data = joblib.load(os.path.join(test_data_dir, client_file))

# Encrypt features
enc_inputs = []
for i in input_data:
    e_i = client.quantize_encrypt_serialize(i.reshape(1, -1))
    enc_inputs.append(e_i)

# Export encrypted features
enc_inputs_file = os.path.join(enc_test_data_dir, client_file)
if not os.path.exists(enc_test_data_dir): 
    os.mkdir(enc_test_data_dir)
joblib.dump(enc_inputs, enc_inputs_file)

# Call cog 
call = subprocess.run(['cog', 'predict', 
                    '-i', f'eval_key_dir=@{keys_dir}/{keys_file}', 
                    '-i', f'enc_input_file=@{enc_inputs_file}'], 
                    capture_output=True, text=True)
print(call)

# Decrypt results
enc_output = joblib.load(enc_output_file)

output_data = []
for e_o in enc_output:
    o = np.argmax(client.deserialize_decrypt_dequantize(e_o), axis=1)
    output_data.append(o)

print(output_data)
print(joblib.load(os.path.join(ground_truths_dir, client_file)))