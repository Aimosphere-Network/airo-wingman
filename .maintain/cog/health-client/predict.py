from typing import Any
from cog import BasePredictor, Input
from concrete.ml.deployment import FHEModelClient
import numpy as np


class Predictor(BasePredictor):
    def setup(self):
        self.client = FHEModelClient("")
        self.client.load()
        self.client.generate_private_and_evaluation_keys()

    def predict(self, symptoms: str = Input(description="Symptoms vector")) -> Any:
        serialized_evaluation_keys = self.client.get_serialized_evaluation_keys()
        user_symptoms = np.fromstring(symptoms, dtype=int, sep=".").reshape(1, -1)
        encrypted_quantized_user_symptoms = self.client.quantize_encrypt_serialize(user_symptoms)
        return {
            "eval_key": serialized_evaluation_keys.hex(),
            "data": encrypted_quantized_user_symptoms.hex()
        }
