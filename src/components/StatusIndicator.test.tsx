import { describe, it, expect } from 'vitest';
import { render, screen } from '../test/utils.tsx';
import { StatusIndicator } from './StatusIndicator';

describe('StatusIndicator', () => {
  it('renders active status with correct label', () => {
    render(<StatusIndicator status="active" />);
    expect(screen.getByText('All delivered')).toBeInTheDocument();
  });

  it('renders pending status with correct label', () => {
    render(<StatusIndicator status="pending" />);
    expect(screen.getByText('Pending')).toBeInTheDocument();
  });

  it('renders error status with correct label', () => {
    render(<StatusIndicator status="error" />);
    expect(screen.getByText('Error')).toBeInTheDocument();
  });

  it('renders unknown status with correct label', () => {
    render(<StatusIndicator status="unknown" />);
    expect(screen.getByText('Loading...')).toBeInTheDocument();
  });

  it('renders an icon for each status', () => {
    const { container: activeContainer } = render(
      <StatusIndicator status="active" />
    );
    expect(activeContainer.querySelector('svg')).toBeInTheDocument();

    const { container: errorContainer } = render(
      <StatusIndicator status="error" />
    );
    expect(errorContainer.querySelector('svg')).toBeInTheDocument();
  });
});
