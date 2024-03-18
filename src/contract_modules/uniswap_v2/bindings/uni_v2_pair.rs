
use ethers::prelude::abigen;

abigen!(IUniswapV2Pair,
r#"[
    function getReserves() external view returns (uint112 reserve0, uint112 reserve1, uint32 blockTimestampLast)
    function token0() external view returns (address)
    function token1() external view returns (address)
    function swap(uint256 amount0Out, uint256 amount1Out, address to, bytes calldata data)
    event Sync(uint112 reserve0, uint112 reserve1)
    event Mint(address indexed sender, uint amount0, uint amount1)
]"#;


IErc20,
r#"[
    function balanceOf(address account) external view returns (uint256)
    function decimals() external view returns (uint8)
    function transfer(address recipient, uint256 amount) external returns (bool)
]"#;

);


