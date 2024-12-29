from cog import BasePredictor, Input
from concrete.ml.deployment import FHEModelServer
import base64


class Predictor(BasePredictor):
    def setup(self):
        self.server = FHEModelServer("")

    def predict(self,
                eval_key: str = Input(description="Evaluation key"),
                data: str = Input(description="Encrypted user data")) -> str:
        serialized_evaluation_keys = base64.standard_b64decode(eval_key)
        encrypted_quantized_user_symptoms = base64.standard_b64decode(data)
        encrypted_output = self.server.run(encrypted_quantized_user_symptoms, serialized_evaluation_keys)
        encrypted_output = base64.standard_b64encode(encrypted_output)
        return encrypted_output.decode("ascii")
