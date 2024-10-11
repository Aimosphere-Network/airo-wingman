from cog import BasePredictor, Input
from concrete.ml.deployment import FHEModelServer


class Predictor(BasePredictor):
    def setup(self):
        self.server = FHEModelServer("")

    def predict(self,
                eval_key: str = Input(description="Evaluation key"),
                data: str = Input(description="Encrypted user data")) -> str:
        serialized_evaluation_keys = bytes.fromhex(eval_key)
        encrypted_quantized_user_symptoms = bytes.fromhex(data)
        encrypted_output = self.server.run(encrypted_quantized_user_symptoms, serialized_evaluation_keys)
        return encrypted_output.hex()
