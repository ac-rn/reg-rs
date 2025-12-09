"""
Basic tests for the reg_parser Python bindings.

These tests verify the basic functionality of the Python bindings.
"""

import pytest
import reg_parser


def test_version():
    """Test that version is available."""
    assert hasattr(reg_parser, '__version__')
    assert isinstance(reg_parser.__version__, str)
    assert len(reg_parser.__version__) > 0


def test_classes_available():
    """Test that all expected classes are available."""
    assert hasattr(reg_parser, 'Hive')
    assert hasattr(reg_parser, 'RegistryKey')
    assert hasattr(reg_parser, 'RegistryValue')
    assert hasattr(reg_parser, 'ValueData')
    assert hasattr(reg_parser, 'ValueType')
    assert hasattr(reg_parser, 'BaseBlock')
    assert hasattr(reg_parser, 'HbinHeader')


def test_hive_open_invalid():
    """Test that opening an invalid file raises an error."""
    with pytest.raises(Exception):
        reg_parser.Hive.open("nonexistent_file.dat")


def test_hive_open_with_logs_invalid():
    """Test that opening with invalid logs raises an error."""
    with pytest.raises(Exception):
        reg_parser.Hive.open_with_logs(
            "nonexistent.dat",
            "nonexistent.LOG1",
            "nonexistent.LOG2"
        )


# Note: The following tests require actual test data files
# Uncomment and modify paths as needed when test data is available

# def test_hive_open_valid():
#     """Test opening a valid hive file."""
#     hive = reg_parser.Hive.open("test_data/SYSTEM")
#     assert hive is not None
#     assert isinstance(hive, reg_parser.Hive)


# def test_base_block():
#     """Test accessing base block information."""
#     hive = reg_parser.Hive.open("test_data/SYSTEM")
#     base_block = hive.base_block()
#     
#     assert isinstance(base_block, reg_parser.BaseBlock)
#     assert isinstance(base_block.signature, str)
#     assert isinstance(base_block.primary_sequence, int)
#     assert isinstance(base_block.secondary_sequence, int)
#     assert isinstance(base_block.root_cell_offset, int)
#     assert isinstance(base_block.hive_bins_data_size, int)


# def test_root_key():
#     """Test accessing the root key."""
#     hive = reg_parser.Hive.open("test_data/SYSTEM")
#     root = hive.root_key()
#     
#     assert isinstance(root, reg_parser.RegistryKey)
#     assert isinstance(root.name(), str)
#     assert isinstance(root.subkey_count(), int)
#     assert isinstance(root.value_count(), int)


# def test_subkeys():
#     """Test enumerating subkeys."""
#     hive = reg_parser.Hive.open("test_data/SYSTEM")
#     root = hive.root_key()
#     subkeys = root.subkeys()
#     
#     assert isinstance(subkeys, list)
#     assert len(subkeys) > 0
#     assert all(isinstance(k, reg_parser.RegistryKey) for k in subkeys)


# def test_values():
#     """Test enumerating values."""
#     hive = reg_parser.Hive.open("test_data/SYSTEM")
#     root = hive.root_key()
#     values = root.values()
#     
#     assert isinstance(values, list)
#     assert all(isinstance(v, reg_parser.RegistryValue) for v in values)


# def test_value_data():
#     """Test accessing value data."""
#     hive = reg_parser.Hive.open("test_data/SYSTEM")
#     root = hive.root_key()
#     values = root.values()
#     
#     if values:
#         value = values[0]
#         assert isinstance(value.name(), str)
#         assert isinstance(value.data_size(), int)
#         
#         data = value.data()
#         assert isinstance(data, reg_parser.ValueData)


# def test_hbins():
#     """Test accessing hbin headers."""
#     hive = reg_parser.Hive.open("test_data/SYSTEM")
#     hbins = hive.hbins()
#     
#     assert isinstance(hbins, list)
#     assert len(hbins) > 0
#     assert all(isinstance(h, reg_parser.HbinHeader) for h in hbins)
#     
#     # Check first hbin
#     hbin = hbins[0]
#     assert isinstance(hbin.signature, str)
#     assert isinstance(hbin.offset, int)
#     assert isinstance(hbin.size, int)


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
