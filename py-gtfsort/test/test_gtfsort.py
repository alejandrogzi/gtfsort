import gtfsortpy
import os
import pandas as pd
import pytest
from typing import Tuple
import unittest

NTHREADS = 4

def get_test_file() -> Tuple[str]:
    """Return the shared input fixture and its prospective sorted path."""
    tmp = gtfsortpy.get_test_file()
    out = tmp + '.sorted'
    return (tmp, out)

def main():
    """Sort the shared fixture and return the extension's status message."""
    tmp, out = get_test_file()
    status = gtfsortpy.sort(tmp, out, NTHREADS)
    return status

class TestGtf(unittest.TestCase):

    def setUp(self):
        """Sort the fixture before each Python API assertion."""
        self.tmp, self.out = get_test_file()
        self.status = gtfsortpy.sort(self.tmp, self.out, NTHREADS)
    
    def test_file_creation(self):
        """Verify that file-based sorting creates the requested output."""
        self.assertTrue(os.path.isfile(self.out))
    
    def test_status_output(self):
        """Verify that file-based sorting returns a status message."""
        self.assertIsNotNone(self.status)

    def test_sorted_file_line_count(self):
        """Verify that sorting retains the expected number of records."""
        expected_line_count = 333875
        with open(self.out, 'r') as f:
            line_count = len(f.readlines())

        self.assertEqual(line_count, expected_line_count)
    
    def test_sorted_file_order(self):
        """Verify natural chromosome ordering in the sorted file."""
        rule = ['GL456221.1', 'chr1', 'chr2', 'chr3', 'chr5', 'chrM']
        chrom_order = pd.read_csv(self.out, sep='\t', usecols=[0], header=None, comments='#')[0].unique().tolist()

        self.assertEqual(chrom_order, rule)
