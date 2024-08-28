import numpy
import joblib
import os
from sklearn.datasets import fetch_openml
from sklearn.model_selection import train_test_split, GridSearchCV
from sklearn.tree import DecisionTreeClassifier
# from concrete.ml.sklearn import DecisionTreeClassifier

from sklearn.utils.validation import check_is_fitted


OPEN_ML_DATASET = 44
TEST_SIZE = 10
TEST_DIR = 'test_data'
GT_DIR = 'ground_truths'

model_file = 'model.pkl'
train_data_file = 'train_data.csv'


"""DATA MANAGEMENT"""

#dataset
features, classes = fetch_openml(data_id=OPEN_ML_DATASET, as_frame=False, cache=True, return_X_y=True)
# labels
classes = classes.astype(numpy.int64)

# Splitting dataset between train and test
x_train, x_test, y_train, y_test = train_test_split(
    features,
    classes,
    test_size=0.15,
    random_state=42,
)

# Dump test data into files
if not os.path.exists(TEST_DIR):
    os.makedirs(TEST_DIR)
    for i in range(0, len(x_test), TEST_SIZE):
        file_path = os.path.join(TEST_DIR, f'test_data{i}-{i+TEST_SIZE}.csv')
        joblib.dump(x_test[i:TEST_SIZE+i], file_path)

# Dump ground truth data into files
if not os.path.exists(GT_DIR):
    os.makedirs(GT_DIR)
    for i in range(0, len(y_test), TEST_SIZE):
        file_path = os.path.join(GT_DIR, f'ground_truth{i}-{i+TEST_SIZE}.csv')
        joblib.dump(y_test[i:TEST_SIZE+i], file_path)

# Dump train data into file
if not os.path.exists(train_data_file):
    # Dump train data into external file
    joblib.dump(x_train, train_data_file)


"""MODEL"""

# List of hyper parameters to tune
param_grid = {
    "max_features": [None, "sqrt", "log2"],
    "min_samples_leaf": [1, 10, 100],
    "min_samples_split": [2, 10, 100],
    "max_depth": [None, 2, 4, 6, 8],
}

# Find best hyper parameters with cross validation
"""
Grid search: systematically trying out every possible combination of hyperparameters within a 
specified range or list of values and evaluating the model's performance for each combination using cross-validation. 
cv: sets the number of folds in cross-validation. 
The training data is split into 10 parts, and the model is trained and validated 10 times, 
each time using a different part as the validation set and the remaining parts as the training set.   
n_jobs: sets the number of jobs to run in parallel. 
n_jobs=1 means the tasks will be run sequentially, n_jobs=-1 would use all available processors.
"""
grid_search = GridSearchCV(
    DecisionTreeClassifier(),
    param_grid,
    cv=10,
    scoring="average_precision",
    error_score="raise",
    n_jobs=1,
)
gs_results = grid_search.fit(x_train, y_train)

# Define model with best hyper parameters
model = DecisionTreeClassifier(
    max_features=gs_results.best_params_["max_features"],
    min_samples_leaf=gs_results.best_params_["min_samples_leaf"],
    min_samples_split=gs_results.best_params_["min_samples_split"],
    max_depth=gs_results.best_params_["max_depth"],
) 

# Train model
model.fit(x_train, y_train)

try:
    check_is_fitted(model)
    print("model fitted.")
except ValueError:
    print("model not fitted.")

# Save the model to a file if it doesn't exist
if not os.path.exists(model_file):
   joblib.dump(model, model_file)
