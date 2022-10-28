import * as React from 'react';
import { Box, Button, Card, Grid, Typography } from '@mui/material';
import { GridColDef, GridRenderCellParams } from '@mui/x-data-grid';
import { SelectChangeEvent } from '@mui/material/Select';
import { useMainContext } from '../../context/main';
import { gatewayToGridRow } from '../../components/Gateways';
import { GatewayResponse } from '../../typeDefs/explorer-api';
import { TableToolbar } from '../../components/TableToolbar';
import { CustomColumnHeading } from '../../components/CustomColumnHeading';
import { Title } from '../../components/Title';
import { cellStyles, UniversalDataGrid } from '../../components/Universal-DataGrid';
import { currencyToString } from '../../utils/currency';
import { Tooltip } from '../../components/Tooltip';

export const PageGateways: React.FC = () => {
  const { gateways } = useMainContext();
  const [filteredGateways, setFilteredGateways] = React.useState<GatewayResponse>([]);
  const [pageSize, setPageSize] = React.useState<string>('50');
  const [searchTerm, setSearchTerm] = React.useState<string>('');

  const handleSearch = (str: string) => {
    setSearchTerm(str.toLowerCase());
  };

  React.useEffect(() => {
    if (searchTerm === '' && gateways?.data) {
      setFilteredGateways(gateways?.data);
    } else {
      const filtered = gateways?.data?.filter((g) => {
        if (
          g.gateway.location.toLowerCase().includes(searchTerm) ||
          g.gateway.identity_key.toLocaleLowerCase().includes(searchTerm) ||
          g.owner.toLowerCase().includes(searchTerm)
        ) {
          return g;
        }
        return null;
      });
      if (filtered) {
        setFilteredGateways(filtered);
      }
    }
  }, [searchTerm, gateways?.data]);

  const columns: GridColDef[] = [
    {
      field: 'owner',
      headerName: 'Owner',
      renderHeader: () => <CustomColumnHeading headingTitle="Owner" />,
      width: 380,
      headerAlign: 'left',
      headerClassName: 'MuiDataGrid-header-override',
      renderCell: (params: GridRenderCellParams) => (
        <Typography sx={cellStyles} data-testid="owner">
          {params.value}
        </Typography>
      ),
    },
    {
      field: 'identity_key',
      headerName: 'Identity Key',
      renderHeader: () => <CustomColumnHeading headingTitle="Identity Key" />,
      headerClassName: 'MuiDataGrid-header-override',
      width: 380,
      headerAlign: 'left',
      renderCell: (params: GridRenderCellParams) => (
        <Typography sx={cellStyles} data-testid="identity-key">
          {params.value}
        </Typography>
      ),
    },
    {
      field: 'bond',
      width: 150,
      type: 'number',
      renderHeader: () => <CustomColumnHeading headingTitle="Bond" />,
      headerClassName: 'MuiDataGrid-header-override',
      headerAlign: 'left',
      renderCell: (params: GridRenderCellParams) => (
        <Typography sx={cellStyles} data-testid="pledge-amount">
          {currencyToString(params.value)}
        </Typography>
      ),
    },
    {
      field: 'host',
      renderHeader: () => <CustomColumnHeading headingTitle="IP:Port" />,
      width: 180,
      headerAlign: 'left',
      headerClassName: 'MuiDataGrid-header-override',
      renderCell: (params: GridRenderCellParams) => (
        <Typography sx={cellStyles} data-testid="host">
          {params.value}
        </Typography>
      ),
    },
    {
      field: 'location',
      renderHeader: () => <CustomColumnHeading headingTitle="Location" />,
      width: 180,
      headerAlign: 'left',
      headerClassName: 'MuiDataGrid-header-override',
      renderCell: (params: GridRenderCellParams) => (
        <Button
          onClick={() => handleSearch(params.value as string)}
          sx={{ ...cellStyles, justifyContent: 'flex-start' }}
          data-testid="location-button"
        >
          <Tooltip text={params.value} id="gateway-location-text">
            <Box
              sx={{
                overflow: 'hidden',
                whiteSpace: 'nowrap',
                textOverflow: 'ellipsis',
              }}
            >
              {params.value}
            </Box>
          </Tooltip>
        </Button>
      ),
    },
  ];

  const handlePageSize = (event: SelectChangeEvent<string>) => {
    setPageSize(event.target.value);
  };

  if (gateways?.data) {
    return (
      <>
        <Title text="Gateways" />
        <Grid container>
          <Grid item xs={12}>
            <Card
              sx={{
                padding: 2,
                height: '100%',
              }}
            >
              <TableToolbar
                onChangeSearch={handleSearch}
                onChangePageSize={handlePageSize}
                pageSize={pageSize}
                searchTerm={searchTerm}
              />
              <UniversalDataGrid rows={gatewayToGridRow(filteredGateways)} columns={columns} pageSize={pageSize} />
            </Card>
          </Grid>
        </Grid>
      </>
    );
  }
  return null;
};
