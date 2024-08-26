import joblib
import pandas as pd
import concrete.ml.sklearn as concrete
from cog import BasePredictor, Input, Path

class Predictor(BasePredictor):
    def setup(self):
        # Load model
        with open("model.pkl", "rb") as file:
            model = joblib.load(file)   
        # Load training data
        with open("train_data.csv", 'rb') as file: 
            train_data = joblib.load(file)
        
        # Convert model into concrete model
        self.model = concrete.DecisionTreeClassifier.from_sklearn_model(model, n_bits=6)

        # Generate circuit from trained model, using training data as shape
        self.circuit = self.model.compile(train_data)
        
        # Generate keys for circuit
        self.circuit.client.keygen(force=True)


    def predict(
        self,
        request: Path = Input(description="pkl file containing features"),
        ) -> str:
        # Load the features from the .pkl file
        df = pd.read_pickle(request)

        # Predict for each request
        predictions = pd.DataFrame()
        for row in df.itertuples(index=True):
            p = self.circuit.run(row)
            df = pd.concat([df, p[0]], ignore_index=True)
        
        return predictions

