import numpy as np
import joblib
from fastapi import FastAPI, UploadFile, File, HTTPException, Form
from pydantic import BaseModel
from typing import List
from concrete.ml.deployment import FHEModelClient

# Initialize FastAPI app
app = FastAPI()

# Model class for input data (if you want to use JSON)
class EncryptionRequest(BaseModel):
    function: str  # encrypt or decrypt
    input_file: str  # path or name of the input file (CSV or enc/bin)


# Endpoint to handle requests
@app.post("/process/")
async def process_data(function: str = Form(...), input_file: UploadFile = File(...)):
    # Generate client from client.zip file (generated while model-owner builds circuit)
    client = FHEModelClient("")

    # Check if input file type and function type are valid
    if function not in ["encrypt", "decrypt"]:
        raise HTTPException(status_code=400, detail="Invalid function type. Use 'encrypt' or 'decrypt'.")
    
    input_data = await input_file.read()

    if function == "encrypt":
        try:
            # Load input from the uploaded file (assuming it's in joblib format)
            input_data = joblib.load(input_file.file)
        except Exception as e:
            raise HTTPException(status_code=400, detail="Invalid input file format for encryption.")

        """Input encryption with evaluation key"""
        # Create private and evaluation keys
        eval_key = client.get_serialized_evaluation_keys()

        # Encrypt features individually
        enc_inputs = []
        for i in input_data:
            e_i = client.quantize_encrypt_serialize(i.reshape(1, -1))
            enc_inputs.append(e_i)

        # Return the serialized evaluation key and encrypted inputs as part of the response
        return {
            "eval_key": eval_key.decode('latin1'),  # To ensure compatibility with JSON
            "encrypted_data": [e_i.decode('latin1') for e_i in enc_inputs]
        }

    elif function == "decrypt":
        try:
            # Decrypt results individually (assuming input is serialized encrypted data)
            dec_inputs = []
            for e_i in input_data:
                i = np.argmax(client.deserialize_decrypt_dequantize(e_i), axis=1)
                dec_inputs.append(i)

            # Return decrypted inputs
            return {
                "decrypted_data": dec_inputs
            }
        except Exception as e:
            raise HTTPException(status_code=400, detail="Invalid input file format for decryption.")


# To run this server:
# Use the following command:
# uvicorn client:app --reload

