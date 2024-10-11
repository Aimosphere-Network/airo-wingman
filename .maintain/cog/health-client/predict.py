from typing import Any
from cog import BasePredictor, Input
from concrete.ml.deployment import FHEModelClient
import numpy as np


class Predictor(BasePredictor):
    def setup(self):
        self.client = FHEModelClient("")
        self.client.load()

    def predict(self,
                encrypt: bool = Input(description="Switch between encryption and decryption"),
                input: str = Input(
                    description="Either a symptoms vector when encrypt mode or an encrypted output when decrypt mode")) -> Any:
        if encrypt:
            self.client.generate_private_and_evaluation_keys()
            serialized_evaluation_keys = self.client.get_serialized_evaluation_keys()
            user_symptoms = np.fromstring(input, dtype=int, sep=".").reshape(1, -1)
            encrypted_quantized_user_symptoms = self.client.quantize_encrypt_serialize(user_symptoms)
            return {
                "eval_key": serialized_evaluation_keys.hex(),
                "data": encrypted_quantized_user_symptoms.hex()
            }
        else:
            encrypted_output = bytes.fromhex(input)
            output = self.client.deserialize_decrypt_dequantize(encrypted_output)
            return {
                "output": output.flatten()
            }
