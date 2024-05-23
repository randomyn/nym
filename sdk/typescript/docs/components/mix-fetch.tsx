import React, { useState } from 'react';
import CircularProgress from '@mui/material/CircularProgress';
import Button from '@mui/material/Button';
import TextField from '@mui/material/TextField';
import Typography from '@mui/material/Typography';
import Box from '@mui/material/Box';
import { mixFetch } from '@nymproject/mix-fetch-full-fat';
import Stack from '@mui/material/Stack';
import Paper from '@mui/material/Paper';
import type { SetupMixFetchOps } from '@nymproject/mix-fetch-full-fat';

const defaultUrl = 'https://nymtech.net/favicon.svg';
const args = { mode: 'unsafe-ignore-cors' };

const mixFetchOptions: SetupMixFetchOps = {
  preferredGateway: '983r9LKDT9UUxx4Zsn2AH49poJ7Ep24ueR8ENfWFgCX6', // with WSS
  preferredNetworkRequester:
    'DxAc9J4eqREc8hYfDobkSc81JLkmmrhJ77zJvHShUPoi.92bnebXtBuwKiYycrpioaAiYgta5hHWkys5aSGBQg5av@983r9LKDT9UUxx4Zsn2AH49poJ7Ep24ueR8ENfWFgCX6',
  mixFetchOverride: {
    requestTimeoutMs: 60_000,
  },
  forceTls: true, // force WSS
};

export const MixFetch = () => {
  const [url, setUrl] = useState<string>(defaultUrl);
  const [html, setHtml] = useState<string>();
  const [busy, setBusy] = useState<boolean>(false);

  const handleFetch = async () => {
    try {
      setBusy(true);
      setHtml(undefined);
      const response = await mixFetch(url, args, mixFetchOptions);
      console.log(response);
      const resHtml = await response.text();
      setHtml(resHtml);
    } catch (err) {
      console.log(err);
    } finally {
      setBusy(false);
    }
  };

  return (
    <div style={{ marginTop: '1rem' }}>
      <Stack direction="row">
        <TextField
          disabled={busy}
          fullWidth
          label="URL"
          type="text"
          variant="outlined"
          defaultValue={defaultUrl}
          onChange={(e) => setUrl(e.target.value)}
        />
        <Button variant="outlined" disabled={busy} sx={{ marginLeft: '1rem' }} onClick={handleFetch}>
          Fetch
        </Button>
      </Stack>

      {busy && (
        <Box mt={4}>
          <CircularProgress />
        </Box>
      )}
      {html && (
        <>
          <Box mt={4}>
            <strong>Response</strong>
          </Box>
          <Paper sx={{ p: 2, mt: 1 }} elevation={4}>
            <Typography fontFamily="monospace" fontSize="small">
              {html}
            </Typography>
          </Paper>
        </>
      )}
    </div>
  );
};
