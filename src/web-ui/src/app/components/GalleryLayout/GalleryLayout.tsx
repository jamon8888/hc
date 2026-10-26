import React from 'react';
import { ScrollArea } from '@alex/ui';
import './GalleryLayout.scss';

interface GalleryLayoutProps extends React.HTMLAttributes<HTMLDivElement> {
  children: React.ReactNode;
  className?: string;
}

const GalleryLayout: React.FC<GalleryLayoutProps> = ({ children, className, ...rootProps }) => (
  <div
    data-alex-component="gallery-layout"
    data-alex-part="root"
    {...rootProps}
    className={['gallery-layout', className].filter(Boolean).join(' ')}
  >
    <ScrollArea className="gallery-layout__body" data-alex-component="gallery-layout" data-alex-part="body">
      <div className="gallery-layout__body-inner" data-alex-component="gallery-layout" data-alex-part="content">
        {children}
      </div>
    </ScrollArea>
  </div>
);

export default GalleryLayout;
export type { GalleryLayoutProps };
