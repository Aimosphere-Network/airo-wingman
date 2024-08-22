import joblib
import pickle
import pandas as pd
import concrete.ml.sklearn as concrete
from cog import BasePredictor, Input, Path

class Predictor(BasePredictor):
    def setup(self):
        # Load the model and training data from the provided .pkl files
        with open("sklearn_model/trained_model.pkl", "rb") as file:
            model = pickle.load(file)   
        with open("sklearn_model/train_data.pkl", 'rb') as file: 
            train_data = pickle.load(file)

        # Compile concrete model into FHE circuit
        try: 
            self.circuit = model.compile(train_data)
            print("FHE Circuit running!")
        except:
            print("FHE Circuit generation failed.")
            try:
                # Use ConcreteDecisionTreeClassifier as a fallback
                self.model = concrete.ConcreteDecisionTreeClassifier.from_sklearn_model(model)
                self.circuit = model.compile(train_data)
                print("FHE Circuit running!")
            except:
                print("FHE Circuit generation failed.")
        
        self.circuit.client.keygen(force=False)


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

