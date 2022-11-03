#!/bin/bash -l
## Example run script for Node Crunch with SLURM

## General configuration options
#SBATCH -J Node_Crunch_Mandel2
#SBATCH -o node_crunch_mandel2.%j.%N.out
#SBATCH -e node_crunch_mandel2.%j.%N_Err.out
#SBATCH --mail-user=my_email@somewhere.com
#SBATCH --mail-type=ALL

## Machine and CPU configuration
## Number of tasks per job:
## Rayon uses all the available cores, so change this to your hardware settings.
#SBATCH -n 8
## Number of nodes:
#SBATCH -N 1

# Ensure that all the binaries are available on all the cluster nodes at the same place.
# Usually this is done in the cluster setup via NFS or some other distributed
# filesystem already.

# change this to the actual ip address of the system where the server is running.
mandel2 --ip ip_of_server
