import os
import sys
import itertools
from subprocess import Popen, PIPE, STDOUT
from tqdm import tqdm

num_instances = [4, 10, 20]
# epoch_lengths = [10, 20]
epoch_lengths = [1, 5, 10]
data_sizes = ['small', 'medium', 'large']

for i, (n, e, d) in tqdm(enumerate(itertools.product(num_instances, epoch_lengths, data_sizes))):
    os.makedirs(f"./logs/{n}_nodes", exist_ok=True)
    print(f"\nStarting iteration {i}. (num_instances={n}, epoch_length={e}, data_size={d})\n")
    p = Popen(['./target/debug/cs244b_project', f'{n}', f'bench', f'{e}', f'{d}', 'benchmark'], stdout=sys.stdout, stderr=sys.stderr)    
    status = p.wait()
    print(f"\nIteration {i} completed with status {status}. (num_instances={n}, epoch_length={e}, data_size={d})\n")