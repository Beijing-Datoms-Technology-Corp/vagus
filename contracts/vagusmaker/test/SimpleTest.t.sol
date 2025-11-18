// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import "forge-std/Test.sol";
import "../SimpleTest.sol";

contract SimpleTestContract is Test {
    SimpleTest simpleTest;

    function setUp() public {
        simpleTest = new SimpleTest();
    }

    function testConsensusLogic() public {
        bool result = simpleTest.testConsensusLogic();
        assertTrue(result, "Consensus logic should work");
    }

    function testHanoiMoves() public {
        uint256 moves3 = simpleTest.getHanoiMoves(3);
        assertEq(moves3, 7, "3-disk Hanoi should have 7 moves");

        uint256 moves4 = simpleTest.getHanoiMoves(4);
        assertEq(moves4, 15, "4-disk Hanoi should have 15 moves");

        uint256 moves10 = simpleTest.getHanoiMoves(10);
        assertEq(moves10, 1023, "10-disk Hanoi should have 1023 moves");

        uint256 moves20 = simpleTest.getHanoiMoves(20);
        assertEq(moves20, 1048575, "20-disk Hanoi should have 1,048,575 moves");
    }
}
