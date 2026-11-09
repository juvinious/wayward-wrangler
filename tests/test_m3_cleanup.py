"""Test harness cleanup must pin and revalidate ownership before sending a signal."""
import signal
import unittest
from unittest.mock import patch
import milestone3

class CleanupOwnership(unittest.TestCase):
    def test_changed_identity_never_signals(self):
        expected={'pid':123,'boot':'boot','start':'old'}
        with patch.object(milestone3,'alive',return_value=True), \
             patch.object(milestone3.os,'pidfd_open',return_value=999), \
             patch.object(milestone3.os,'close') as close, \
             patch.object(milestone3,'identity',return_value={**expected,'start':'new'}), \
             patch.object(milestone3.signal,'pidfd_send_signal') as send:
            with self.assertRaises(AssertionError):milestone3.signal_owned(expected,signal.SIGTERM)
            send.assert_not_called();close.assert_called_once_with(999)

    def test_dead_identity_never_opens_or_signals_pid(self):
        with patch.object(milestone3,'alive',return_value=False), \
             patch.object(milestone3.os,'pidfd_open') as open_pid:
            milestone3.signal_owned({'pid':123},signal.SIGTERM)
            open_pid.assert_not_called()
