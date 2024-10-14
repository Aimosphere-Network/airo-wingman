import numpy as np
from fastapi import FastAPI
from pydantic import BaseModel
from concrete.ml.deployment import FHEModelClient

COG_MODEL_URL = "http://localhost:5001/predictions" 

app = FastAPI()


"""Evaluation key"""

# Generate client from client.zip file (generated while model-owner builds circuit)
client = FHEModelClient("")
# Create private and evaluation keys
eval_key = client.get_serialized_evaluation_keys()


"""Encryption"""

class EncryptionRequest(BaseModel):
    input_data: list  # List of values for encryption

# Encryption Endpoint
@app.post("/encrypt/")
async def encrypt_data(request: EncryptionRequest):
    # Encrypt features individually
    enc_inputs = []
    for i in request.input_data:
        e_i = client.quantize_encrypt_serialize(i.reshape(1, -1))
        enc_inputs.append(e_i)

    return {
        "eval_key": eval_key.decode('latin1'),
        "enc_input": [e_i.decode('latin1') for e_i in enc_inputs]
    }


"""Decryption"""

class DecryptionRequest(BaseModel):
    enc_input_data: list  # List of encrypted results for decryption

# Decryption Endpoint
@app.post("/decrypt/")
async def decrypt_data(request: DecryptionRequest):
    # Decrypt results individually (assuming input is serialized encrypted data)
    dec_inputs = []
    for e_i in request.enc_input_data:
        i = np.argmax(client.deserialize_decrypt_dequantize(e_i), axis=1)
        dec_inputs.append(i)

    return {
        "dec_prediction": dec_inputs
    }


# To run this server:
# Use the following command:
# uvicorn client:app --reload
